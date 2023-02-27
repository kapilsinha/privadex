# Concurrency Coordinator Utilities
Phat Contract is run by a decentralized pool of workers. There needs to be explicit coordination amongst the workers to avoid concurrency issues e.g. double spending, failed execution due to a worker crashing, etc.

## Requirements
The below is one set of sufficient requirements to ensure no concurrency issues. There are many other implementations that can also work.
1. ExecutionSteps on a chain are assigned consecutive nonces. Any worker running this ExecutionStep must use this nonce. There can be no skipped or duplicate nonces. If an ExecutionStep is dropped (i.e. it never gets included in a finalized block due to network congestion or an error in the transaction), the associated nonce is reclaimed so that the subsequent nonces that have already been assigned remain valid.
2. An ExecutionPlan is executed on at most one worker, at any point in time.

Note: Use the `--debug` flag and examine logs to auto-generate the HTTP body from the CLI equivalent.

## NonceManager
We order the cases roughly in order of most to least likely. By design they should carry out the same logic regardless of order (assuming no change in state).
Note 1: BlockAtLastConfirmedNonce and ExecStepPendingBlockAdded are not needed for the current update logic, but we keep it for a better audit trail (and in case we use it for the logic down the road).
Note 2: While a case is atomic, running cases in sequence is not - so it is possible albeit unlikely that none of the cases succeed (and we will need to retry on the next iteration).

```bash
## Boolean states: IsExecutionStepAssigned, IsDroppedNoncesEmpty, IsPendingTxnsEmpty

# Case 1: Cold start / cleanup
# When: IsPendingTxnsEmpty (and thus !IsExecutionStepAssigned)
aws dynamodb update-item --table-name privadex_phat_contract --key file://astar_key.json --update-expression "SET BlockAtLastConfirmedNonce = :curblock, DroppedNonces = :emptylist, ExecStepPendingNonce = :pendingnonce, ExecStepPendingBlockAdded = :pendingblockadded, NextNonce = :nextnonce" --condition-expression "size(ExecStepPendingNonce) = :zero" --expression-attribute-values '{":curblock": {"N":"1001"}, ":emptylist": {"L": []}, ":pendingnonce": {"M":{"execstep_0xcase1": {"N":"50"}}}, ":pendingblockadded": {"M":{"execstep_0xcase1": {"N":"1000"}}}, ":nextnonce": {"N":"51"}, ":zero": {"N":"0"}}' --return-values NONE

# Case 2: Assign the next nonce
# When: !IsExecutionStepAssigned AND IsDroppedNoncesEmpty AND !IsPendingTxnsEmpty
aws dynamodb update-item --table-name privadex_phat_contract --key file://astar_key.json --update-expression "SET ExecStepPendingBlockAdded.execstep_0xcase2 = :curblock, ExecStepPendingNonce.execstep_0xcase2 = NextNonce, NextNonce = NextNonce + :one" --condition-expression "attribute_not_exists(ExecStepPendingNonce.execstep_0xcase2) AND size(DroppedNonces) = :zero AND size(ExecStepPendingNonce) > :zero" --expression-attribute-values '{":curblock":{"N":"1001"}, ":one": {"N":"1"}, ":zero": {"N":"0"}}' --return-values UPDATED_NEW
# Example output:
{
    "Attributes": {
        "ExecStepPendingBlockAdded": {
            "M": {
                "execstep_0xcase2": {
                    "N": "1001"
                }
            }
        },
        "ExecStepPendingNonce": {
            "M": {
                "execstep_0xcase2": {
                    "N": "51"
                }
            }
        },
        "NextNonce": {
            "N": "52"
        }
    }
}

# Case 3: Pull the existing assignment for the ExecutionStep
# When: IsExecutionStepAssigned (and thus !IsPendingTxnsEmpty)
aws dynamodb get-item --table-name privadex_phat_contract --key file://astar_key.json --projection-expression "ExecStepPendingNonce.execstep_0xcase3, ExecStepPendingBlockAdded.execstep_0xcase3"
# Example output:
{
    "Item": {}
}
or
{
    "Item": {
        "ExecStepPendingNonce": {
            "M": {
                "execstep_0xcase2": {
                    "N": "51"
                }
            }
        },
        "ExecStepPendingBlockAdded": {
            "M": {
                "execstep_0xcase2": {
                    "N": "1001"
                }
            }
        }
    }
}

# Case 4: Reclaim a dropped transaction's nonce
# When: !IsExecutionStepAssigned AND !IsDroppedNoncesEmpty AND !IsPendingTxnsEmpty
aws dynamodb update-item --table-name privadex_phat_contract --key file://astar_key.json --update-expression "SET ExecStepPendingBlockAdded.execstep_0xcase4 = :curblock, ExecStepPendingNonce.execstep_0xcase4 = DroppedNonces[0] REMOVE DroppedNonces[0]" --condition-expression "attribute_not_exists(ExecStepPendingNonce.execstep_0xcase4) AND size(DroppedNonces) > :zero AND size(ExecStepPendingNonce) > :zero" --expression-attribute-values '{":curblock":{"N":"1001"}, ":zero": {"N":"0"}}' --return-values UPDATED_NEW
# Example output:
{
    "Attributes": {
        "ExecStepPendingNonce": {
            "M": {
                "execstep_0xcase4": {
                    "N": "32"
                }
            }
        },
        "ExecStepPendingBlockAdded": {
            "M": {
                "execstep_0xcase4": {
                    "N": "1001"
                }
            }
        }
    }
}


# For every case: A transaction has been finalized
aws dynamodb update-item --table-name privadex_phat_contract --key file://astar_key.json --update-expression "SET BlockAtLastConfirmedNonce = :curblock REMOVE ExecStepPendingBlockAdded.execstep_0xcase1, ExecStepPendingNonce.execstep_0xcase1" --expression-attribute-values '{":curblock":{"N":"1001"}}' --return-values NONE

# For every case: A transaction has been dropped
# The conditional update should not be necessary if only one worker is assigned to the ExecutionPlan
# but we add it for safety (since we use a list and not a set for DroppedNonces)
aws dynamodb update-item --table-name privadex_phat_contract --key file://astar_key.json --update-expression "SET DroppedNonces = list_append(DroppedNonces, :droppednonce) REMOVE ExecStepPendingBlockAdded.execstep_0xcase1, ExecStepPendingNonce.execstep_0xcase1" --condition-expression "attribute_exists(ExecStepPendingBlockAdded.execstep_0xcase1)" --expression-attribute-values '{":droppednonce":{"L":[{"N":"53"}]}}' --return-values NONE
```

## ExecutionPlanAssigner
```bash
# Allocate a plan to a worker
# When: isallocated = false OR updateepochmillis is old (1 minute)
aws dynamodb update-item --table-name privadex_phat_contract --key file://execplans_key.json --update-expression "SET WorkerIsAllocated.execplan_0xplan1 = :true, WorkerAssignmentUpdateEpochMillis.execplan_0xplan1 = :epochmillis ADD Plans :plan" --condition-expression "WorkerIsAllocated.execplan_0xplan1 = :false OR WorkerAssignmentUpdateEpochMillis.execplan_0xplan1 < :minepochmillis" --expression-attribute-values '{":true": {"BOOL":true}, ":epochmillis": {"N": "1060000"}, ":plan": {"SS":["0xplan1"]}, ":false": {"BOOL":false}, ":minepochmillis": {"N": "1000000"}}' --return-values NONE

# Unallocate a plan from a worker
# This can also be used to 'register' an ExecutionPlan since we ADD Plans :plan
# When: Unconditional update. No other worker can allocate before this (and only a worker that has allocated last should unallocate)
aws dynamodb update-item --table-name privadex_phat_contract --key file://execplans_key.json --update-expression "SET WorkerIsAllocated.execplan_0xplan1 = :false, WorkerAssignmentUpdateEpochMillis.execplan_0xplan1 = :epochmillis ADD Plans :plan" --expression-attribute-values '{":false": {"BOOL":false}, ":epochmillis": {"N": "1000000"}, ":plan": {"SS":["0xplan1"]}}' --return-values NONE

# Remove exec plan from processing queue (when the exec plan is terminated i.e. confirmed/dropped/failed)
aws dynamodb update-item --table-name privadex_phat_contract --key file://execplans_key.json --update-expression "REMOVE WorkerIsAllocated.execplan_0xplan1, WorkerAssignmentUpdateEpochMillis.execplan_0xplan1 DELETE Plans :plan" --expression-attribute-values '{":plan": {"SS":["0xplan1"]}}' --return-values NONE

# Get list of active ExecutionPlans. To be used by the scheduler/driver to assign ExecutionPlans to workers
aws dynamodb get-item --table-name privadex_phat_contract --key file://execplans_key.json --projection-expression "Plans"
# Example output:
{
    "Item": {
        "Plans": {
            "SS": [
                "0x01010101010101010101010101010101",
                "0x02020202020202020202020202020202",
                "0x04040404040404040404040404040404"
            ]
        }
    }
}
```

## Prestart txn de-duplicate
A malicious user can try to use the same prestart txn for multiple cross-chain swaps. We enforce that there is just one prestart step per execution plan.
```bash
aws dynamodb update-item --table-name privadex_phat_contract --key file://prestart_txns_key.json --update-expression "SET LastUpdateEpochMillis = :epochmillis ADD TxnHash :txnhashset" --condition-expression "NOT contains(TxnHash, :txnhash)" --expression-attribute-values '{":epochmillis": {"N": "1070000"}, ":txnhashset": {"SS":["0xdbcdef"]}, ":txnhash": {"S":"0xdbcdef"}}' --return-values NONE
```