# Ely advanced charts: design and implementation plan

## Goal and constraints

Ely needs interactive charts for both ordinary dashboard data and large time
series. Keep the existing chart components and add the liveplot-based
`AdvancedChart` for continuous, high-volume data. Study gpui-d3rs for chart
composition and mathematical ideas; implement any needed behavior in Ely's
own code. Do not copy its source or add it as a dependency.

The existing liveplot integration is uncommitted work. Its MIT notice is in
`src/charts/advanced/LICENSE.gpui-liveplot`. It is adapted from the user's fork
at commit `ba3bdbe72ab1480bee4420c5d2f2178ce76913dd`.

## Design

Keep two paths until evidence supports combining them:

* Existing Ely charts cover categorical and other dashboard chart types, using
  their current Ely elements, theme and data models.
* `AdvancedChart` owns a keyed `PlotView` for continuous line and scatter data.
  It shares a series store and its min-max summaries across renders. Keep the
  viewport, selection and touch input in the persistent view.

Call `Series::handle()` in declarative render code. It preserves the series ID
and shares its storage. `share()` creates a distinct series ID; `clone()`
copies the data. The caller must notify the view after an external append.
Use a revision when chart configuration changes, and verify the precise
update behavior before depending on dynamic axes or themes.

Do not adopt a second GPUI renderer or replace the min-max summary tree with a
full scan. The gpui-d3rs default feature set selects a different GPUI stack;
whether its no-default-features build resolves alongside Ely is not established.
The distinction does not affect this decision to implement concepts locally.

## Delivery sequence

1. Stabilize the liveplot wrapper: stable series identity, update and removal
   behavior, selection, callbacks, theme changes, pan and pinch. Add focused
   regression tests. Preserve the license notice.
2. Inventory existing Ely chart primitives against the desired use cases.
   Specify missing scale and shape behavior before implementing it; reuse Ely's
   existing helpers where they meet the behavior. Add a small chart end-to-end
   only when it fills a demonstrated gap.
3. Add chart types incrementally, with examples and tests for empty, invalid,
   missing and large data as relevant. Avoid promising ECharts parity.
4. Measure a reproducible million-point pan/zoom workload and record hardware,
   data shape, timings and memory. Verify desktop, iOS and Android interactions
   before claiming platform support. Include narrow-screen tooltips.

## Open risks and acceptance

Stable identity alone does not prove that data, axis, theme or link changes
propagate correctly; test each observable contract. Verify missing samples
remain gaps and selection events return original data coordinates. Compare
fresh rendering with a keyed rerender after updates, and test actual gestures
on each supported platform. No million-point performance or mobile result is
assumed from unit tests. Changes stay uncommitted until reviewed.

## First checkpoint: stable handle

Scope: `src/charts/advanced/series.rs` defines `Series::handle(&self) -> Series`;
`examples/gallery/pages/charts/cartesian.rs` and
`src/charts/advanced/tests.rs` pass that handle to `AdvancedChart`. Its ID and
store are shared, while `share()` creates a new ID and `clone()` copies storage.
The `handle_preserves_identity_and_shares_appends` test must show that appending
through the original changes the handle's generation without changing its ID.
`AdvancedChart` (`src/charts/advanced/element.rs`) also tracks series
generations so external appends redraw without resetting manual zoom, preserves
caller `SeriesId`s across rebuilds, resets custom themes when cleared, passes
`reset_token` through `reset_view_in_window`, and avoids legend/tooltip overlap
on narrow plots (`src/charts/advanced/backend/frame/hud.rs`).

## Native d3rs-concept charts and scales

Implemented natively in Ely without adding `gpui-d3rs` as a dependency or
copying its source:

* **Scales (`src/charts/scale.rs`, `src/charts/advanced/axis.rs`,
  `transform.rs`, `interaction.rs`)**: `Log(base)` and `Symlog(constant)`
  scales with nice decade/symmetric ticks, wired into `LineChart`, `AreaChart`,
  `BarChart`, `ScatterChart`, and `AdvancedChart` (including log/symlog pan &
  zoom in scale space).
* **`HexbinChart` (`src/charts/hexbin.rs`)**: O(N) pointy-top hexagonal lattice
  binning in plot pixel space with density shading, hover tooltip, and SVG/CSV.
* **`ContourChart` (`src/charts/contour.rs`)**: 2D Gaussian KDE and explicit
  scalar-field Marching Squares contour bands and isolines with hover readout.
* **`RidgelineChart` (`src/charts/ridgeline.rs`)**: stacked overlapping KDE
  curves reusing `stats::density` and `stats::five`.
* **`DifferenceChart` (`src/charts/difference.rs`)**: two-series comparison
  with crossing-point splitting shaded `success` vs `danger`.
* **`PolarChart` (`src/charts/polar.rs`)**: Nightingale rose / radial sector
  chart with concentric scale rings and share tooltips.
* **`CirclePacking` (`src/charts/packing.rs`)**: nested packed circles sharing
  `sunburst::Slice`.
* **`AreaChart::stream()` and `ScatterChart::voronoi()`**: centered
  streamgraph stacking (`stream_stacked`) and clipped Voronoi tessellation cells
  with nearest-site picking.
* **Curves and Symbols (`src/charts/series.rs`, `paint.rs`, `export.rs`)**:
  `Curve` (`Linear`, `Monotone`, `Basis`, `Cardinal`, `Step`, `StepBefore`,
  `StepAfter`) and `Symbol` (`Circle`, `Square`, `Diamond`, `Triangle`, `Cross`,
  `Star`) across canvas rendering and SVG export.
* **`HorizonChart` & `RadialLineChart` (`src/charts/horizon.rs`)**: multi-band
  mirrored/offset horizon strips and polar cyclic line/envelope charts.
* **`TreeChart` (`src/charts/hierarchy_tree.rs`)**: tidy tree and cluster
  dendrograms in orthogonal and radial layouts (`TreeLayoutMode`).
* **`GeoChart` (`src/charts/geo.rs`)**: `Equirectangular`, `Mercator`,
  `Orthographic` (3D rotation + hemisphere clipping), and `Stereographic`
  projections with graticules, choropleth regions, and point markers.
* **`SurfaceChart` & `ParallelSetsChart` (`src/charts/surface.rs`)**:
  depth-sorted 3D scalar surface meshes and multi-dimension alluvial ribbons.

## Desktop and mobile/tablet interaction parity

* **Direct manipulation across charts**: orbit drag + wheel/pinch zoom + reset
  on `GeoChart` and `SurfaceChart`; continuous domain drag/touch pan +
  wheel/pinch zoom + `Show all` reset on `ScatterChart`, `HexbinChart`,
  `ContourChart`, and `RidgelineChart`; subtree tap drilldown + hole/outside/`Up`
  return on `Sunburst` and `CirclePacking`; internal-node collapse/expand +
  pan/zoom on `TreeChart`; brush/pinch zoom, swipe pan, tap-to-pin, dial
  rotation, and `ChartLegend` series toggles on `HorizonChart`,
  `RadialLineChart`, `DifferenceChart`, `PolarChart`, and `ParallelSetsChart`.
* **Mobile/tablet ergonomics**: every `on_pinch` clears stale single-finger
  drag/brush state; `theme.touch_mode` enforces >= 24 px radius (48 px diameter)
  hit targets on discrete points/nodes and 48x48 px reset/legend controls;
  tooltips flip at 55% container width on narrow 240-390 px cards.

## Remaining checkpoints

* `PlotView::refresh` provides explicit imperative refresh, and rerendering
  `AdvancedChart` detects `Series::generation()` changes automatically; add a
  dedicated `from_view` `reset_token` + `selected_x` rerender test if needed.
* Extend native d3rs-inspired primitives only where Ely lacks coverage (for
  example log/symlog scales or 2D density contours) with per-chart specs first.
* Add a reproducible benchmark before setting a performance threshold: one
  million monotonic points, a fixed viewport and a scripted pan/zoom trace;
  record device, build profile, warm-up, frame times and memory. Compare with
  the fork on the same device and data.
* Exercise selection, pan, pinch and tooltip placement on desktop, an iOS
  simulator and Android, including a narrow screen. Record device and OS and
  any failures in the validation report. Simulator success is not a real-device
  result.

These are checkpoints, not yet a complete specification for additional chart
types. Select the first missing chart from the inventory and define its input,
edge cases, tests and files before implementing it.
