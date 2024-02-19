# [Security Event Token (SET)](https://tools.ietf.org/html/rfc8417) generation

## Note
Token generation has moved to helper functions in module `helpers::set_tokens`. For reference, the following steps were used to generate the token files on the command line and are kept here for reference.

## Manual token generation
Install [jq](https://stedolan.github.io/jq/) and [jwt](https://github.com/mike-engel/jwt-cli).

Create a file e.g. `set_token_delete_user.json` and run:

```sh
jq -r tostring set_token_delete_user.json | jwt encode -S @../rumba-test.pem -A RS256 -k TEST_KEY - > set_token_delete_user.txt
```
