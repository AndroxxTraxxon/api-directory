#!/bin/bash

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
PROJECT_ROOT="$(dirname -- "${SCRIPT_DIR}" )"
# Define the directory where the SSL files will be stored
SSL_DIR="$PROJECT_ROOT/.ssl.dev"


# Check if the SSL directory exists, if not create it
if [ ! -d "$SSL_DIR" ]; then
    mkdir -p "$SSL_DIR"
fi

# Generate a new RSA private key and certificate signing request
openssl req -new -newkey rsa:4096 -nodes -keyout "$SSL_DIR/snakeoil.key" -out "$SSL_DIR/snakeoil.csr"

# Sign the certificate signing request with the private key to create the certificate
openssl x509 -req -sha256 -days 365 -in "$SSL_DIR/snakeoil.csr" -signkey "$SSL_DIR/snakeoil.key" -out "$SSL_DIR/snakeoil.pem"
