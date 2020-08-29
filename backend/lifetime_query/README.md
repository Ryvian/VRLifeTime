# vrlifetime-backend
The backend of VRLifetime, a plugin to visualize the lifetime of objects and assist in bug finding in VSCode for Rust

## 1. Usage:

```./run.sh ${TARGET_DIRECTORY}```

The tool generates json files "lifetime_${CRATE_NAME}.info" under the ${TARGET_DIRECTORY},
which include the variable id, span, and the lifetime ranges.

e.g.
```
./run.sh examples/vec-uaf
```

```./query.sh ${JSON_QUERY_STR}```

${JSON_QUERY_STR} includes "root", "file", "pos", e.g.

```
"{\"root\":\"/home/boqin/Projects/HackRust/vrlifetime-backend/examples/vec-uaf\",\"file\":\"src/main.rs\",\"pos\":\"4:9: 4:10\"}"
```
N.B. The escape character is a must.

The tool will search the "lifetime_${CRATE_NAME}.info" under the given root directory.

Then it will parse these json files and search for the variable of the given span.

Finally it will merge the lifetime ranges of the variable and returns the json string, e.g.

```
{
        "/home/boqin/.rustup/toolchains/nightly-2020-05-10-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/src/liballoc/macros.rs":"46:9: 46:39",
        "src/main.rs":"8:5: 8:23, 4:13: 7:6, 4:9: 4:10, 9:1: 9:2, 12:8: 15:2"
}
```





