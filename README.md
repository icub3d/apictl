[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)
[![crates.io](https://img.shields.io/crates/v/apictl.svg)](https://crates.io/crates/apictl)
[![docs.rs](https://img.shields.io/docsrs/apictl/latest)](https://docs.rs/apictl/latest)

# apictl

`apictl` is a command-line application for interacting with APIs. See
the documentation below or run `apictl help` for details.

# Installation

Binaries are available via [releases](https://github.com/icub3d/apictl/releases).

You can also install it using cargo:

```bash
cargo install apictl
```

# Getting Started

## Example 

This repository includes an example [.apictl.yaml](.apictl.yaml)
file. It interacts with [Typicode's
`json-server`](https://github.com/typicode/json-server). You can
install it via npm and then run it in some other shell:

```bash
npm install -g json-server
json-server http://jsonplaceholder.typicode.com/db
```

You should now be able to clone this repository run the examples
within it:

```bash
git clone https://github.com/icub3d/apictl
cd apictl
apictl requests list
apictl requests run -c local get-posts get-user-from-first-post
apictl requests run -c local new-post get-new-post
```

## From Scratch

If you have an existing API you'd like to use, start by creating a
[configuration](#configuration) for it. It is common for the
configuration to be in the repository in which the API is created.

Next, create [contexts](#contexts) for the various environments that
you'll be using.

Then create [requests](#requests) for any API calls you are interested
in running.

Finally, do, some testing on your configuration before publishing your
code.

# Configuration

You can look at [.apictl.yaml](.apictl.yaml) for an example
configuration. Configurations are defined using
[YAML](https://yaml.org/). By default, `apictl` will look for a file
named `.apictl.yaml` in the current directory.

If your project is large enough, you can use a folder to store
multiple configuration files. To do this, run `apictl` with the flag
`--config CONFIG` where CONFIG is the path to your folder. Folder are
read in sorted order and configuration are merged. When information
overlaps (e.g. two requests with the same name) the later overwrites
the former.

Details on values within the configuration can be found below.

# Contexts

Contexts are ways to define variables for your API calls. Within your
configurations, they are created under the `contexts` key. Each
context is a map that contains the variables and the values to
replace. For example, suppose you want to differentiate the URL to use
in requests for your local, dev, staging, and prod environments. You
might do it like this:

```yaml
contexts:
  local:
    base_url: http://localhost:3000
  dev:
    base_url: https://dev.app
  staging:
    base_url: https://staging.app
  prod:
    base_url: https://my-cool-api.com
```

You can now use the variable `base_url` in your requests and it will
be replaced with the value based on the context given. If you want to
run a request using dev, you'd add a flag `--context dev`.

Currently, all variables within the request are replaced if they are
strings. If the body contains strings, they are replaced. File
references will have their path replaced but not the actual content.

## Variables

Once you have your contexts defined, you can using them in your
configuration using the pattern `${name}` where name is the key in the
context.

## Multiple Contexts

You can use multiple contexts which are merged in a similar fashion to
configurations. This may be useful when you have multiple interactions
that may not overlap. For example, you might have multiple
authentication mechanisms you'd like to test for the same API. You can
use a context for the environment and then for the authentication
method.

To use multiple contexts, simply add multiple context flags to your
command: `--context basic-auth --context local`.

# Requests

Requests are API endpoints and the information necessary to make the
requests. They are They also contain basic information like `tags` and
`descriptions` that can be used for searching for requests.

Within your configurations, they are created under the `requests`
key. They contain configuration for including the request method, url,
query parameters, headers, and the body. All but the body is
self-explanatory. Here is an example request:

```yaml
requests:
  get-posts:
    tags: [get]
    description: a simple get
    url: "${base_url}/posts"
    query_parameters:
      _limit: "${pagination_limit}"
```

## Body

The request body can come in several forms. This section describes
each of them in detail and how to use them. Each body type is
described by the `type` field.

### Form

If you just want to submit key/value pairs, You can use the "form"
`type` and it will POST a `application/x-www-form-urlencoded`. An example of this:

```yaml
requests:
  new-post:
    tags: [post]
    description: an example post using the form type
    url: "${base_url}/posts"
    method: POST
    body:
      type: form
      data:
        name: World
        age: 4.543 billion years
```

### Multi-part Form

If you have files you want to send, you'll probably want to use the
multipart format. It works similar to the From but allows you to
specify each field's type, You use the "multipart" `type` and it will
POST a multi-part message. An example of this:

```yaml
requests:
  new-post:
    tags: [post]
    description: an example post using the multipart type
    url: "${base_url}/posts"
    method: POST
    body:
      type: multipart
      data:
        name:
          type: text
          data: World
        age:
          type: text
          data: 4.543 billion years
        avatar:
          type: file
          path: ./avatar.png
```

The "file" `type` will expect a file at the given `path` and put it's
contents in the form submission.

### Raw

If you just want to submit a raw body, use the "raw" `type`. The data
can come from text or a file and will send the contents as the
body. An example of using text:

```yaml
requests:
  new-post:
    tags: [post]
    description: an example post using the raw type
    url: "${base_url}/posts"
    method: POST
    body:
      type: raw
      from:
        type: text
        data: |
          {
            "userId": 1,
            "title": "test post",
            "body": "This is a test post to the json server."
          }
```

And an example of using a file:

```yaml
requests:
  new-post:
    tags: [post]
    description: an example post using the raw type
    url: "${base_url}/posts"
    method: POST
    body:
      type: raw
      from:
        type: file
        path: ./new-post-body.json
```
