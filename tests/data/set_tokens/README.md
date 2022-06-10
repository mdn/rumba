# [Security Event Token (SET)](https://tools.ietf.org/html/rfc8417) generation

Install [jq](https://stedolan.github.io/jq/) and [jwt](https://github.com/mike-engel/jwt-cli).

Create a file e.g. `set_token_delete_user.json` and run:

```sh
jq -r tostring set_token_delete_user.json | jwt encode -S @rumba-test.pem -A RS256 -k TEST_KEY - > set_token_delete_user.txt
```

