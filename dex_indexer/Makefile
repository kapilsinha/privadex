process: migrate
	@node -r dotenv/config lib/processor.js

pairs:
	@node -r dotenv/config lib/pairsProcessor.js

serve:
	@npx squid-graphql-server


migrate:
	@npx squid-typeorm-migration apply


migration:
	@npx squid-typeorm-migration generate

clean_migration:
	rm db/migrations/*.js
	@npx squid-typeorm-migration generate

build:
	@npm run build


codegen:
	@npx squid-typeorm-codegen


typegen:
	@make explore
	@npx squid-substrate-typegen ./typegen/typegen.json


#explore:
#	@npx squid-substrate-metadata-explorer \
#		--chain wss://wss.api.moonriver.moonbeam.network \
#		--archive https://moonriver.archive.subsquid.io/graphql \
#		--out ./typegen/versions.jsonl


up:
	@docker-compose up -d


down:
	@docker-compose down

restart: down build up clean_migration process


.PHONY: process serve start codegen migration migrate up down typegen pairs
