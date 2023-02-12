# PrivaDEX Common Module
This crate defines shared types and utilities.

## Running unit tests
```bash
cargo test --features=s3-live-test -- --nocapture
cargo test --features=dynamodb-live-test -- --nocapture
```
Specifying the features above is critical! Otherwise tests will be filtered out.
If the `s3-live-test` feature is enabled, the corresponding unit tests will attempt
to interact with an actual S3 store. To do so, you need to set the environment vars
S3_ACCESS_KEY and S3_SECRET_KEY beforehand!
Similar if the `dynamodb-live-test` feature is enabled - you need to set the environment
vars DYNAMODB_ACCESS_KEY and DYNAMODB_SECRET_KEY!

## Docker testing guide
To run tests from a Docker container, start the Docker container (instructions in the root README file):
```bash
root@<container-id>:/privadex/dex_aggregator/common# cargo test
```