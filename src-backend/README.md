# rscord Backend

## Setup

### Firebase API keys

Download your api keys from [Firebase](https://console.cloud.google.com/iam-admin/serviceaccounts/details/111845915893473513925/keys?authuser=0&hl=en-US&project=rscord-c2cc1) 
<br>
Rename the file to "fcm-service-account.json" and move it to the root of this project

### Running in docker
```sh
cp example.env .env
mkdir db && echo password > db/password.txt

docker compose up --build -d
```

