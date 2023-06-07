This fun toy simulates a bunch of semitransparent, reflective marbles under gravity and partially
inelastic collisions!

Configuration is done by editing constants (and the ray recursion depth in the fragment shader) and
recompiling.

The default for `nix run` is to start a local webserver. Marble Gravity works through webgl2, kind
of. It works on some people's computers in firefox. I should revisit once webgpu is a(n
established) thing.

Some screenshots:
<p align="center">
  <img src="/screenshots/sun.png" />
  <img src="/screenshots/early.png"/>
  <img src="/screenshots/blob.png"/>
</p>
