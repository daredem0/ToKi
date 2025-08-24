
#!/usr/bin/env python3
import json, math, random, pathlib, argparse
from typing import List, Tuple

def load_atlas(atlas_path: str):
    with open(atlas_path, "r") as f:
        atlas = json.load(f)
    tile_names = set(atlas["tiles"].keys())
    required = {"grass", "water", "sand", "dirt", "stone", "brick", "roof"}
    missing = required - tile_names
    if missing:
        print(f"[warn] Missing tiles: {missing}")
    return atlas, tile_names

def clamp(v, lo, hi):
    return max(lo, min(hi, v))

def neighbors4(x,y,W,H):
    for dx,dy in ((1,0),(-1,0),(0,1),(0,-1)):
        nx,ny=x+dx,y+dy
        if 0<=nx<W and 0<=ny<H:
            yield nx,ny

def draw_river(grid, rng, min_width=2, max_width=4):
    H=len(grid); W=len(grid[0])
    y = rng.randint(H//4, 3*H//4)
    x = 0
    width = rng.randint(min_width, max_width)
    while x < W:
        width = clamp(width + rng.choice([-1,0,0,1]), min_width, max_width)
        y = clamp(y + rng.choice([-1,0,0,1]), 1, H-2)
        for dy in range(-width//2, width - width//2):
            yy = clamp(y+dy, 0, H-1)
            grid[yy][x] = "water"
        x += 1
    # sand banks
    for y in range(H):
        for x in range(W):
            if grid[y][x] == "water":
                for nx,ny in neighbors4(x,y,W,H):
                    if grid[ny][nx] != "water":
                        grid[ny][nx] = "sand"
    return y

def place_mountains(grid, rng, density=0.5):
    H=len(grid); W=len(grid[0])
    for y in range(H):
        for x in range(W):
            band_center = (x / W)
            d = abs((y/H) - band_center)
            chance = max(0.0, 0.35 - d) * 2.0 + (rng.random()-0.5)*0.1
            if chance > 0.3 and rng.random() < density:
                grid[y][x] = "stone"

def carve_road_h(grid, y, x0, x1, width=2):
    H=len(grid); W=len(grid[0])
    y0 = clamp(y - width//2, 0, H-1)
    y1 = clamp(y + (width-1)//2, 0, H-1)
    for yy in range(y0, y1+1):
        for x in range(max(0,x0), min(W-1,x1)+1):
            if grid[yy][x] != "water":
                grid[yy][x] = "dirt"

def carve_road_v(grid, x, y0, y1, width=2):
    H=len(grid); W=len(grid[0])
    x0 = clamp(x - width//2, 0, W-1)
    x1 = clamp(x + (width-1)//2, 0, W-1)
    for xx in range(x0, x1+1):
        for y in range(max(0,y0), min(H-1,y1)+1):
            if grid[y][xx] != "water":
                grid[y][xx] = "dirt"

def place_house(grid, top_left, size=4):
    H=len(grid); W=len(grid[0])
    x0,y0 = top_left
    x1 = x0+size-1
    y1 = y0+size-1
    if x1>=W or y1>=H: 
        return False
    for y in range(y0,y1+1):
        for x in range(x0,x1+1):
            if grid[y][x] in ("water","sand","dirt"):
                return False
    for x in range(x0,x1+1):
        grid[y0][x] = "brick"
        grid[y1][x] = "brick"
    for y in range(y0,y1+1):
        grid[y][x0] = "brick"
        grid[y][x1] = "brick"
    for y in range(y0+1,y1):
        for x in range(x0+1,x1):
            grid[y][x] = "roof"
    return True

def place_town(grid, rng, avoid_y=None):
    H=len(grid); W=len(grid[0])
    tw = max(12, W//2)
    th = max(12, H//2)
    if avoid_y is not None:
        top_dist = abs(avoid_y - (1 + th//2))
        bot_dist = abs(avoid_y - (H - th - 1 + th//2))
        y0 = 1 if top_dist > bot_dist else (H - th - 1)
    else:
        y0 = 1
    x0 = rng.randint(1, max(1, W - tw - 1))
    x1 = x0 + tw - 1
    y1 = y0 + th - 1

    for y in range(y0,y1+1):
        for x in range(x0,x1+1):
            if grid[y][x] != "water":
                grid[y][x] = "grass"

    road_w = 2
    carve_road_h(grid, y0+road_w, x0, x1, width=road_w)
    carve_road_h(grid, y1-road_w, x0, x1, width=road_w)
    carve_road_v(grid, x0+road_w, y0, y1, width=road_w)
    carve_road_v(grid, x1-road_w, y0, y1, width=road_w)
    house_size = 4
    gap = 2
    step = house_size + gap
    r = y0 + road_w + step
    while r < y1 - road_w - 1:
        carve_road_h(grid, r, x0, x1, width=road_w)
        r += step
    c = x0 + road_w + step
    while c < x1 - road_w - 1:
        carve_road_v(grid, c, y0, y1, width=road_w)
        c += step
    placed=0
    for y in range(y0+road_w+1, y1-road_w-house_size+2, step):
        for x in range(x0+road_w+1, x1-road_w-house_size+2, step):
            if place_house(grid, (x,y), size=house_size):
                placed+=1
    return {"area": (x0,y0,tw,th), "houses": placed}

def make_map(width, height, include_river, include_mountains, include_town, atlas_path, seed):
    rng = random.Random(seed)
    atlas, tile_names = load_atlas(atlas_path)
    grid = [["grass" for _ in range(width)] for _ in range(height)]
    river_mid_y = None
    if include_river:
        river_mid_y = draw_river(grid, rng)
    if include_mountains:
        place_mountains(grid, rng, density=0.55)
    town_info=None
    if include_town:
        town_info = place_town(grid, rng, avoid_y=river_mid_y)
    tile_list = [t for row in grid for t in row]
    out = {
        "size": [width, height],
        "tile_size": atlas.get("tile_size", [8,8]),
        "atlas": str(pathlib.Path(atlas_path).name),
        "tiles": tile_list
    }
    return out, {"river_row": river_mid_y, "town": town_info}

def main():
    ap = argparse.ArgumentParser(description="Procedural tilemap generator")
    ap.add_argument("--width", type=int, required=True)
    ap.add_argument("--height", type=int, required=True)
    ap.add_argument("--atlas", type=str, required=True, help="Path to terrain atlas JSON (e.g., terrain.json)")
    ap.add_argument("--seed", type=int, default=0)
    ap.add_argument("--river", action="store_true", help="Include river")
    ap.add_argument("--mountains", action="store_true", help="Include mountains")
    ap.add_argument("--town", action="store_true", help="Include town")
    ap.add_argument("--out", type=str, required=True)
    args = ap.parse_args()

    data, meta = make_map(args.width, args.height, args.river, args.mountains, args.town, args.atlas, args.seed)
    with open(args.out, "w") as f:
        json.dump(data, f, indent=2)
    print(f"Wrote {args.out}")
    print(json.dumps(meta))

if __name__ == "__main__":
    main()
