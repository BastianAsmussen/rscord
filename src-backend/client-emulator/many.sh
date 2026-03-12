#!/usr/bin/env bash

set -e

SERVER="http://localhost:8080"
NUM_USERS=10
VERBOSE=""

usage() {
	echo "Usage: $0 [-n NUM_USERS] [-s SERVER] [-v]"
	exit 1
}

while [[ $# -gt 0 ]]; do
	case "$1" in
	-n | --num)
		NUM_USERS="$2"
		shift 2
		;;
	-s | --server)
		SERVER="$2"
		shift 2
		;;
	-v | --verbose)
		VERBOSE="-v"
		shift
		;;
	*)
		usage
		;;
	esac
done

random_string() {
	tr -dc 'a-z0-9' </dev/urandom | head -c "$1"
}

for ((i = 1; i <= NUM_USERS; i++)); do
	HANDLE="user$(random_string 6)"
	EMAIL="${HANDLE}@example.com"
	PASSWORD="Pass$(random_string 10)"

	echo "Creating user $i/$NUM_USERS: $HANDLE"

	./client-emulator \
		--server "$SERVER" \
		--email "$EMAIL" \
		--handle "$HANDLE" \
		--password "$PASSWORD" \
		$VERBOSE
done

echo "Done creating $NUM_USERS users."
