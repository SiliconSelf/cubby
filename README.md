# `cubby`

A parquet-backed matrix homeserver

## Building

Cubby is built with `cargo build`. It has minimal external library dependencies, but being a web app it does depend on libssl.

## FAQ

### Why Is This Called Cubby

Parquet, the file format serving as the datastore for this project, is also a French word meaning "a small space." Cubby conveys a similar meaning in English.

### Can I Use This?

If you really want to.

### Should I Use This?

Absolutely Not.

### Why Does This Exist?

This project exists for two main reasons:

1. I work with [polars](https://pola.rs) professionally at [Zelis](https://zelis.com) and one day had the extraordinarily cursed idea of making a columnar matrix homeserver. After that it was just a matter of seeing if I could, regardless of if I should.
2. I am friends with the developers of [Grapevine](https://gitlab.computer.surgery/matrix/grapevine-fork), a fork of [Conduit](https://conduit.rs). I have wanted to contribute more to the fork, but the source code of Conduit is largely incomprehensible to me as someone who hasn't been contributing to it for a long time. By starting my own server from scratch, I hope to create some things like [Structured Error Handling](https://github.com/SiliconSelf/cubby/blob/3a13b5c02789b448ba5a6a4889a4d65e3afe21ac/cubby_server/src/api/client/accounts/get_username_availability.rs#L17) along the way that are helpful to cherry-pick for a more serious attempt at a matrix homeserver.
