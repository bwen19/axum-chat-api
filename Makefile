DATABASE_URL=postgres://root:secret@localhost:5432/chat

postgres:
	docker run --name postgres -p 5432:5432 -e POSTGRES_USER=root -e POSTGRES_PASSWORD=secret -d postgres:15.2-alpine3.17

createdb:
	docker exec -it postgres createdb --username=root --owner=root chat

dropdb:
	docker exec -it postgres dropdb chat

schema:
	dbml2sql --postgres -o ./schema/schema.sql ./schema/chat.dbml

sqlx:
	cargo install sqlx-cli --no-default-features --features native-tls,postgres

migration:
	sqlx migrate add -r chat --source ./migrations

migrateup:
	DATABASE_URL=${DATABASE_URL} sqlx migrate run --source ./migrations

migratedown:
	DATABASE_URL=${DATABASE_URL} sqlx migrate revert --source ./migrations

prepare:
	cargo sqlx prepare

server:
	cargo run

container:
	docker build . -t eruhini2022/chat-server

.PHONY: postgres createdb dropdb schema sqlx migration migrateup migratedown prepare server container
