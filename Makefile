include .env

postgres:
	docker run --name postgres -p 5432:5432 -e POSTGRES_USER=${POSTGRES_USER} \
		-e POSTGRES_PASSWORD=${POSTGRES_PASSWORD} -d postgres:16.1-alpine3.19

createdb:
	docker exec -it postgres createdb --username=${POSTGRES_USER} \
		--owner=${POSTGRES_USER} ${POSTGRES_DB}

dropdb:
	docker exec -it postgres dropdb ${POSTGRES_DB}

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
	docker run --name redis -p 6379:6379 -d redis:7.2-alpine3.19 \
		redis-server --requirepass ${REDIS_HOST_PASSWORD}

container:
	docker build . -t eruhini2022/chat-api

.PHONY: postgres createdb dropdb schema migration migrateup migratedown \
	prepare redis server container
