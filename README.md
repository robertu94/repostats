# Repo stats

Collects download statistics from Github.  Requires a personal access token with Administration Rights (read-only).


## Installation

Install sqlite3 and openssl libraries, pkgconfig, and cargo.

```
cargo build
```

# Configuration

## GitHub

Creating an access token:

1. Go to Github.com
2. If you haven't yet, [enable Fine Grained access tokens for your organization following the Github instructions](https://docs.github.com/en/organizations/managing-programmatic-access-to-your-organization/setting-a-personal-access-token-policy-for-your-organization)
3. Click your avatar in the top right hand corner
4. Click Settings
5. Click Developer Settings
6. Click Personal Access Tokens
7. Click Fine Grained Tokens
8. Click Generate New Token
9. Give it a token name like "ProjectName Traffic Collector"
10. Give it an expiration date of 364 days from now (longest option)
11. Select the owner as the organization.
12. Give it access to all repositories
13. Under permissions, repository permissions give it Administration: Read-Only (this will also set metadata to read-only)
14. Then click generate token

Once you've generated the token, create a config file using  `config_example.json` as an example.

## Other

Future download statistics can be added in the future.
