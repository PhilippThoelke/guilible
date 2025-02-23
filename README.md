<h1 align="center">🙃 guilible 🙃</h1>
<h3 align="center">—: the gui library that believes in you :—</h3>


> [!CAUTION]
> ### !!! VERY EARLY STAGE OF DEVELOPMENT !!!
> #### This library is not yet ready for use. It lacks most features, is not stable and more of a testing ground at this point.

## Usage
For now the Python API is very simple and just creates a window with a hardcoded stress test.

```python
import guilible as gl

win = gl.Window()
win.start()
```

## Building wheels
Eventually we will build the Rust code in CI and publish Python wheels, for this the maturin-generated CI script will be a good starting point: `maturin generate-ci github`

For now we can locally build the wheel simply by running `pip install -e .` in the repository root.