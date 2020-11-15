This fun toy simulates a bunch of semitransparent, reflective marbles under gravity and partially
inelastic collisions!

Configuration is done by editing constants (and the ray recursion depth in the fragment shader) and
recompiling. To build, make sure you have the build and run-time dependencies listed in `flake.nix`
sorted and use cargo for Rust stable.

I have unfortunately not managed to produce a working build of this for webgpu/the browser, but I
hope to do so in the future.

Some screenshots:
<p align="center">
  <img src="/screenshots/sun.png" />
  <img src="/screenshots/early.png"/>
  <img src="/screenshots/blob.png"/>
</p>
