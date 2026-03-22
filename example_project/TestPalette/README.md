# TestPalette

Small palette-rendering demo project.

What it shows:
- left: indexed player using atlas default palette (`gb_default`)
- middle: indexed NPC with per-entity palette override (`night`)
- right: true-color control sprite unaffected by indexed palette changes

Useful checks:
1. Open the project in the editor and inspect `Project -> Palettes`.
2. Inspect the authored palette files in `palettes/`.
3. Change `Global Indexed Override` and run the runtime.
4. Verify the left and middle sprites recolor, while the right sprite stays true-color.
5. Open `assets/sprites/indexed_demo.json` in the sprite editor and inspect `Color Mode`.

Run from the workspace root:

```bash
cargo run -p toki-runtime -- --project-path example_projects/TestPalette --scene "Main Scene"
```
