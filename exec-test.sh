#!/usr/bin/env bash
# Usage:
# ./exec-test.sh [endpoint] [code]
# Runs an exec test against the specified api sercer with the provided code.

set -e

SERVER=${1:-"http://localhost:3000"}

# read into CODE until EOF
CODE=$(cat)
# if its empty add the default
if [ -z "$CODE" ]; then
    CODE='fn main() -> i32 { return 0; }'
fi

jobId=$(curl -s -X POST "$SERVER/api/v1/execute" \
    -H "Content-Type: application/json" \
    -d "$(jq -n --arg task execute --arg code "$CODE" '{task: $task, code: $code}')" | jq -r '.jobId')

echo "Job ID: $jobId"

set +e
for i in {1..30}; do
    echo "Checking for result (attempt $i)..."

    # Wait until you get anything besides a http error
    curl "$SERVER/api/v1/results/$jobId" -s -o /dev/null -w "%{http_code}" | grep -q "200"
    if [ $? -eq 0 ]; then
        result=$(curl -s "$SERVER/api/v1/results/$jobId")
        echo "Result: $result"
        exit 0
    fi

    sleep 1
done
