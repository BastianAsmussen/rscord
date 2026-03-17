# rscord Backend

## Setup

### Firebase API keys

Download your API keys from [Firebase](https://console.cloud.google.com/iam-admin/serviceaccounts/details/111845915893473513925/keys?authuser=0&hl=en-US&project=rscord-c2cc1).  
Rename the file to `fcm-service-account.json` and move it to the [backend root](./).

### Environment

```sh
cp example.env .env

mkdir db && echo password > db/password.txt
```

## Running

### Docker

```sh
docker compose up --build -d
```

### Testing Locally

The API endpoint tests require a PostgreSQL instance. A test-specific Compose
file is provided:

```sh
docker compose -f compose.test.yaml up -d

cargo test

docker compose -f compose.test.yaml down
```
