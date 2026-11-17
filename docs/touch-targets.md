# Touch targets

The 48 pixel floor guarantees a hit area, not a layout. Apply it to controls whose visual size carries no spatial meaning, such as buttons, list rows, inputs, tabs, map markers, and canvas editing handles.

Do not enlarge elements whose size or position encodes data, such as map pin tips, strokes, heat-map cells, chart glyphs, selection overlays, and canvas content. Use an explicit minimum or a padded transparent hit shape instead.

The floor preserves larger minima. Rem minima use the crate's 16 pixel per rem convention. Fractional minima remain unchanged because their resolved size depends on the parent. A caller that knows the resolved size can use `Theme::touch_minimum` directly.

For `CanvasTouch::update`, send all active contacts after each change, including an empty snapshot on release. Send the final position before removing a contact; `End` uses its last supplied position. Invalid coordinates or duplicate IDs cancel the sequence. Three or more contacts block it until release. Pinch scale is incremental, not cumulative. Do not feed the same contacts to GPUI gestures at the same time.
