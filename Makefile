postgres:
	docker run --name postgres -p 5432:5432 -e POSTGRES_USER=root -e POSTGRES_PASSWORD=secret -d postgres:15.4-alpine3.18

createdb:
	docker exec -it postgres createdb --username=root --owner=root chat

dropdb:
	docker exec -it postgres dropdb chat

schema:
	dbml2sql --postgres -o ./schema/chat.sql ./schema/chat.dbml

migration:
	sqlx migrate add -r $(name)

migrateup:
	sqlx migrate run

migratedown:
	sqlx migrate revert

prepare:
	cargo sqlx prepare

redis:
	docker run --name redis -p 6379:6379 -d redis:7.2-alphine3.18

server:
	cargo run

container:
	docker build . -t eruhini2022/chat-server

.PHONY: postgres createdb dropdb schema migration migrateup migratedown prepare redis server container
