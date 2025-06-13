#!/bin/bash

# Set the withdrawal amount and destination address
withdrawal_amount=$1
destination_address=$2
public_key=$3
endpoint=${4:-"http://127.0.0.1:50051"}  # Optional endpoint with default value

# Check if the withdrawal amount is provided
if [ -z "$withdrawal_amount" ]; then
  echo "Error: Please provide a withdrawal amount."
  exit 1
fi

# Check if the destination address is provided
if [ -z "$destination_address" ]; then
  echo "Error: Please provide a destination address."
  exit 1
fi

# Check if the public key is provided
if [ -z "$public_key" ]; then
  echo "Error: Please provide a public key."
  exit 1
fi

# Run the propose withdrawal command and extract the challenge
propose_output=$(cargo run --bin "withdrawal" -- propose "$withdrawal_amount" "$destination_address" "$public_key" --endpoint "$endpoint" 2>&1)
echo "$propose_output"

# Extract challenge from the output
challenge=$(echo "$propose_output" | grep "Challenge:" | sed 's/Challenge: //')

# Check if challenge is empty
if [ -z "$challenge" ]; then
  echo "Error: Could not extract challenge from propose withdrawal command output."
  exit 1
fi

echo "Challenge: $challenge"

# Sign the challenge using the CLI
signature_output=$(cargo run --bin "cli" -- sign "$challenge" 2>&1)
echo "$signature_output"
signature=$(echo "$signature_output" | grep "signature" | sed 's/.*signature: "\([^"]*\)".*/\1/')

# Check if signature is empty
if [ -z "$signature" ]; then
  echo "Error: Could not extract signature from sign command output."
  exit 1
fi

echo "Signature: $signature"

# Confirm the withdrawal with the signature
confirm_output=$(cargo run --bin "withdrawal" -- confirm "$challenge" "$signature" --endpoint "$endpoint" 2>&1)
echo "$confirm_output"

# Check if the confirmation was successful
if echo "$confirm_output" | grep -q "Success: true"; then
  echo "Withdrawal confirmed successfully"
else
  echo "Error: Withdrawal confirmation failed"
  exit 1
fi 