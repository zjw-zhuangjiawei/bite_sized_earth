# Coordinate Mapping

Three coordinate spaces meet in this project: the **sim grid** (2D), the **.ron model** (3D, Z-up), and the **Bevy world** (3D, Y-up). This doc pins the conventions so models, sim, and render stay aligned.

## Spaces at a glance

| Space | X | Y | Z | Up | Form |
|---|---|---|---|---|---|
| Sim grid | column | row | — | — | 2D `i32` pairs, row-major storage |
| .ron model | left-right | depth | up | **Z** | 3D `i32` per voxel unit |
| Bevy world | right | up | forward | **Y** | 3D `f32` in `Transform` |

The 2D plane of the model (X, Y) is **identical** to sim (X, Y). Model Z is the vertical axis, mapped onto Bevy Y by the mesher.

## Sim grid (`bse_sim`)

- Type: `pub type Cell = (i32, i32);` and `GridPosition { x, y }` (`crates/bse_sim/src/world.rs`, `components.rs`).
- Storage: `index = y * width + x` (row-major; `y` is the row).
- `GridDirection`: `PosX`, `PosY`, `NegX`, `NegY`.
  - `facing_offset`: `PosX = (1,0)`, `PosY = (0,1)`, `NegX = (-1,0)`, `NegY = (0,-1)`.
  - `rotate_cw` / `rotate_ccw` rotate the four cardinals in the sim XY plane.
- `GridSize { right, forward }` is the footprint relative to the facing direction.
- `get_footprint` builds the occupied cells. Anchor is the back-left corner from the facing POV.

## .ron model (`bse_model`, `assets/models/*.ron`)

- Format: face-list. Each `Face { plane, depth, geo_min, geo_size, uv_min, uv_size, material }`.
- 6 planes: `PosX`, `NegX`, `PosY`, `NegY`, `PosZ`, `NegZ`. **Z is up.**
  - Top face: `PosZ`. Bottom face: `NegZ`.
  - Front/back: `PosY` / `NegY`. Left/right: `PosX` / `NegX`.
- `plane_axes(plane) -> (n_axis, u_axis, v_axis)`: maps each plane to (X, Y, Z) indices.
  - `PosX / NegX → (0, 1, 2)` (n=X, u=Y, v=Z)
  - `PosY / NegY → (1, 2, 0)` (n=Y, u=Z, v=X)
  - `PosZ / NegZ → (2, 0, 1)` (n=Z, u=X, v=Y)
- `geo_min` and `geo_size` are voxel-unit integer rectangles on the in-plane (U, V) axes.
- `uv_min` / `uv_size` are pixel-space in the texture.
- Authoring rule: a model that is 64 wide × 32 deep × 20 tall is written in bbox `X ∈ [0, 64]`, `Y ∈ [0, 32]`, `Z ∈ [0, 20]`. The footprint on the floor is its (X, Y) projection.

## Bevy mesh build (`bse_model/src/mesher.rs`)

- `permute([x, y, z]) = [y, z, x]`. Maps model `(X, Y, Z)` → Bevy `(Y, Z, X)`.
- Net effect at the mesh level:
  - Model +X → Bevy +Z
  - Model +Y → Bevy +X
  - Model +Z → Bevy +Y (up)
- `build_mesh` runs `expand_quad` per face, reverses winding for `Neg*` planes so the front side faces the -N direction.
- Texture UVs are normalized to `[0, 1]` by the material's image size (`normalize_uv`).

## Bevy world placement (`bse_shell/src/reactive.rs`)

A cell is a bevy **region**, not a point. Sim cell `(a, b)` is the bevy rectangle
`b..b+1` in bevy.x × `a..a+1` in bevy.z, with its **center** at
`bevy(b+0.5, *, a+0.5)`. Two conventions name two different points of the same
region, both valid:

| Site | Transform point | Bevy coord | Why |
|---|---|---|---|
| Entity (`reactive.rs`) | cell **SW corner** | `(b, 0, a)` | Model asset's local `(0, 0, 0)` is a bbox corner, not a centroid. A 1×1 model placed here fills the cell. |
| Env tile (`environment.rs`) | cell **center** | `(b+0.5, -0.1, a+0.5)` | `Cuboid::new(1.0, 0.2, 1.0)` — transform is the cuboid's center; cuboid center = cell center. |
| Gizmo (`debug_gizmos.rs`) | cell **center** | `(b+0.5, *, a+0.5)` | Gizmo visualizes the cell, not the transform anchor. |

All three cover the same bevy region. They differ in which point of the region
the transform names.

### Entity placement pattern

```rust
Transform::from_xyz(pos.y as f32, 0.0, pos.x as f32)
    .with_scale(Vec3::splat(scale))
    .with_rotation(direction.to_bevy_quat()),
```

- Translation swizzle: `Bevy.x = sim.y`, `Bevy.z = sim.x`, `Bevy.y = 0` (up offset). No `+0.5` — the entity's transform sits at the cell's SW corner.
- Rotation: `to_bevy_quat` maps `GridDirection` to a 90°-step quaternion around Bevy Y:
  - `PosX → 0` (identity)
  - `PosY → +90°`
  - `NegX → 180°`
  - `NegY → -90°`
- Scale is set to `1 / N` where `N` matches the model's grid-cell footprint (e.g. `1/16` for a 16-tile-wide model, `1/32` for a 32-tile-wide model).
- Env tiles in `bse_shell/src/environment.rs` use the cuboid convention:
  `from_xyz(z + 0.5, -0.1, x + 0.5)`. Cuboid center = cell center.
- Camera frames grid center at Bevy `(16, 0, 16)`, isometric.

### Gizmo plotting rule

Gizmos are visual cell indicators, not transform-anchor markers. Center every
gizmo shape on the **cell center**, not the SW corner:

```rust
let cx = sim_x as f32 + 0.5;
let cy = sim_y as f32 + 0.5;
let half = 0.5;
let corners = [
    Vec3::new(cy - half, y, cx - half),
    Vec3::new(cy + half, y, cx - half),
    Vec3::new(cy + half, y, cx + half),
    Vec3::new(cy - half, y, cx + half),
];
```

Same `+0.5` shift for path points, arrows, and slot rects. Skipping the offset
puts the gizmo's bbox on the previous cell (one-cell mismatch with the env tile).

## Combined at identity rotation (facing `PosX`)

With no Y-axis rotation, the model's intrinsic axes line up with the sim axes:

| World direction | Source |
|---|---|
| World +X = sim +Y | model +Y |
| World +Z = sim +X | model +X |
| World +Y (up) | model +Z |

So a model with its "front" face authored along its `+Y` plane faces sim `+Y` at identity rotation. With `GridDirection::PosY` facing (+90° around Bevy Y), the model's `+Y` plane ends up pointing at sim `+Y` (the intended facing direction).

## Worked example: stove

`assets/models/stove.ron` bbox: `X ∈ [0, 64]`, `Y ∈ [0, 32]`, `Z ∈ [0, 20]`. Scale `1/32`.

At sim cell `(3, 5)` facing `PosX` (identity rotation):
- World placement: `Bevy = (5.0, 0.0, 3.0)`, scale `(1/32, 1/32, 1/32)`.
- Model corner `(0, 0, 0)` → world Bevy `(5.0, 0.0, 3.0)`.
- Model corner `(64, 0, 0)` (model +X edge at floor) → world Bevy `(5.0, 0.0, 5.0)` — extends 2 cells in sim +X.
- Model corner `(0, 32, 0)` (model +Y edge at floor) → world Bevy `(6.0, 0.0, 3.0)` — extends 1 cell in sim +Y.
- Model corner `(0, 0, 20)` (top) → world Bevy `(5.0, 0.625, 3.0)` — counter height 0.625 units.

If the same stove is rotated to face `PosY`:
- World placement: `Bevy = (5.0, 0.0, 3.0)`, rotation `+90°` around Bevy Y.
- The model's `+Y` face (back in model terms) ends up facing sim `+X` (perpendicular to the intended facing). To keep the stove's "front" facing the customer in sim `+Y`, the model's front must be authored along `+X` for `PosX` facing, then rotated by the `GridDirection` quaternion to align with the actual placement direction.

## Authoring rules

- Model coordinate space is **Z-up**, with `+X` right and `+Y` depth (forward when `PosX` facing, identity rotation).
- A model's footprint on the floor is the (X, Y) projection of its bbox.
- Place a model at sim cell `(x, y)` with `GridDirection::PosX` by default. The renderer will swizzle and rotate to match.
- Scale `1/N` where `N` is the number of voxels the model spans along its widest floor axis. Match the value to the `GridSize` you give the sim entity.
- Use `Plane::PosZ` / `NegZ` for the top/bottom of the model. Never put a top face on `PosY` or `PosY`-facing axis.

## Quick reference

```
sim (x, y)              →  Bevy (y, 0, x)              # entity transform = cell SW corner
cell (x, y) region      =  Bevy [y, y+1] × [x, x+1]    # the cell itself is a region
cell (x, y) center      =  Bevy (y+0.5, *, x+0.5)      # env tile, gizmo center here
model (X, Y, Z)         →  mesh (Y, Z, X)              # permute at build time
model Z                 =  Bevy Y                      # up, height
model X                 =  sim X (= Bevy Z)            # left-right
model Y                 =  sim Y (= Bevy X)            # depth / forward at identity rot
```
