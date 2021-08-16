# Slimshader

> Shadertoy for WebGPU

A simple command line tool to help developers experiment with the WebGPU shader language.
It compiles and executes a fragment shader and displays the result on your screen.
Slimshader will reload the shader if the file changes. Errors are logged to the console.

## Usage

Build binairy from source and install in `PATH`.

```bash
cargo install --git https://github.com/wrnrlr/slimshader
```

Start slimshader.

```bash
slimshader example/sphere.wgsl
```

Alternativly clone the repository and run it from there.

```bash
cargo run -- example/sphere.wgsl
```
