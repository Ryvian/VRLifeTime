VRLifeTime
--------
A VS Code plugin to visualize lifetime in Rust programs to help avoid concurrency and memory bugs, and providing double lock detection information.
Now for linux only.


## Usage

1. Open a Rust project folder in VSCode (where the .toml file exists)

2. Open a Rust source file in VSCode; select the object you want to visualize; save the file (ctrl+S) to update diagnostic information

## Install

1. install `rustup` from [rust-lang](https://www.rust-lang.org/)

1. download .vsix file from the release.

2. `code --install-extension <path-to-.vsix-file>`



## Development

### install

1. install node.js

1. download this project

2. 
```
$ cd THIS_DIRECTORY
$ npm install
$ npm run compile
```

3. open this folder in VS Code and press `F5`. select `VS Code development (preview)`.

## Misc

The tool currently does not intend to visualize lifetime of references.
