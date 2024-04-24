# API Directory

This package is an API Gateway written in Rust, supported by a 
[browser-based UI](https://github.com/AndroxxTraxxon/api-directory-ui)
written in React

## Prerequisites

- The Rust build toolchain and package manager (Cargo)
  - Follow instructions at [rustup.rs](https://rustup.rs/) to install this tooling
  - This project was last built using Rustc/Cargo version 1.77.2 (stable), but I don't believe it uses anything particularly fringe in the language, so earlier builds should work as well.
- [NodeJS](https://nodejs.org/en/download)
  - Built using version 18
- [SurrealDB](https://surrealdb.com/docs/surrealdb/introduction/start)
- OpenSSL (likely already installed)


## Getting started

### Initializing Git Submodule(s)

This project includes a Submodule for the Web UI components. This delineation
separates the languages between the repositories because the UI leverages React/JS
for its presentation. Initializing the submodule can be done as follows
```bash
git submodule update --init --recursive --remove && git submodule foreach git checkout main
```

### Building the User Interface

> This step requires the git submodule to be initialized.

> This step requires that NodeJS be installed (last built with version 18)


See the readme within the
[UI project](https://github.com/AndroxxTraxxon/api-directory-ui) for detailed 
instructions. For a high-level build, follow these steps:

```bash
cd webui
npm install
npm run build-dist
```

This generate a collection of static files located in the `www` directory
that will be served by the gateway to service the User Interface.


### X.509 TLS Certificates

> Running the included generate-certs.sh script will require that `openssl` is
installed on your machine, and accessible to the current user.

Running this project requires that an X.509 Certificate be available for use 
to leverage TLS for HTTPS communications. At a future time, this location may
be made configurable, but for now, they need to positioned relative to the 
present working directory like so:

- PEM-formatted RSA Private Key at `.ssl.dev/snakeoil.key`
- X.509-formatted Certificate, at `.ssl.dev/snakeoil.pem`

For convenience [a script](scripts/generate-certs.sh) has been written to automate
the generation of self-signed certificates in the correct place, relative to the root
of this project directory.

### Building the Gateway

> This step will require that the Rust package manager, Cargo, 
and the [Rust toolchain](https://rustup.rs/) are installed.

To build the project, simply run `cargo build` for an unoptimized dev version,
or `cargo build --release` for an optimized release build of the application.

The gateway has been verified to build correctly on MacOS and Linux,
for the x84_64 architecture.

### MacOS Additional Prerecs
To run the gateway on a MacOS machine, you'll need to add a loopback Alias
for the IP Address 127.0.2.1 like this:

```bash
sudo ifconfig lo0 alias 127.0.2.1
```

### Linux Optional Requirements
Because the gateway binds to port 443, which is protected, the gateway will normally
be required to run as the root user. Using a utility like 
[authbind](https://manpages.ubuntu.com/manpages/xenial/man1/authbind.1.html) can
alleviate some of this by granularly allowing access on specific ports
for specific users.

### Setting up Host alias
For the User interface to correctly operate, you'll need to add the following entry
to your hosts file:
```
127.0.2.1	apigateway.local
```

### Running the Gateway

To start the gateway service (after building), run the following command:
```bash
sudo target/release/api-directory
```

Alternatively, the debug build can be more directly built and run using cargo:
```bash
sudo cargo run
```

> See the [section above](#linux-optional-requirements) about `authbind` for Linux runtime if you'd like to avoid using
`sudo` to run the program bound to port 443.

### Setting up the first (Admin) User

> This step will require leveraging the SurrealDB directly.
Normal operation does not require the separate SurrealDB application,
but the inital User setup does. SurrealDB should be downloaded and
installed for these next steps. Follow the 
[SurrealDB Installation steps](https://surrealdb.com/docs/surrealdb/installation/)
for your system of choice to continue.

After running the Gateway for the first time, the Database will be created in
the `temp.speedb` directory. Then, run the SurrealDB server on the
database files.
```bash
surreal start file:temp.speedb
```

In a separate terminal, open a SQL Shell client, and insert the Admin role and User.
```bash
surreal sql --ns api_directory --db services

api_directory/services> INSERT INTO role { namespace: "Gateway", name: "Admin" }
api_directory/services> INSERT INTO gateway_user { username: "admin" }
```

At this point, there will be a Role Id and a User ID shown in the SQL Shell, like this:
- for role: `role:wadr6xbtms53q660ho9v`
- for user: `gateway_user:uiqnmjhfp7fl3cs9d18a`

The last step will be to relate the user as a member of the role.
```bash 
api_directory/services> RELATE gateway_user:uiqnmjhfp7fl3cs9d18a->memberOf->role:wadr6xbtms53q660ho9v
```

To check that the records have been correctly inserted, you can query the database
for the related records, and it should return the records:
```bash
api_directory/services> select username, ->memberOf->role.namespace, ->memberOf->role.name FROM gateway_user

[[{ "->memberOf": { "->role": { name: ['Admin'], namespace: ['Gateway'] } }, username: 'admin' }]]
```

Then, close both the Surreal SQL shell and shut down the SurrealDB server.

### (Re-) Setting A User's password

- Go to the [web portal](https://127.0.2.1) and select "Reset it here" from the login page.
- Enter the desired username to reset in the `Username` field and select "Reset Password"
- Switch to the terminal where the service is running, and watch for the reset token message
in the terminal, like so:
```
[2024-04-24T03:42:56Z DEBUG api_directory::auth::repo] New password reset c9k1zsftwki8q1hxinj0 created for user 4sw7q3z0w8ao6jtnzph7
```
- Take the reset token ID and append it to the reset password url, like so:
  - `https://apigateway.local/reset-password/c9k1zsftwki8q1hxinj0`
- In the form, provide the username, password, and password confirmation to reset the password.
- After resetting, a banner message should appear notifying the user that the password has been
  successfully reset.