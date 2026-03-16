# rscord Backend

### Running in Docker

```sh
cp example.env .env
mkdir db && echo password > db/password.txt

docker compose up --build -d
```

### Running tests locally

The API endpoint tests require a PostgreSQL instance. A test-specific
Compose file is provided:

```sh
docker compose -f compose.test.yaml up -d

cargo test

docker compose -f compose.test.yaml down
```
