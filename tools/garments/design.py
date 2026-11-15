# A GarmentCode design from a base design and settings by path: the words-to-garment step's output format.
#   python design.py BASE.yaml OUT.yaml meta.upper=Shirt sleeve.length=1.0 collar.component.style=Hood2Panels ...
import sys, yaml

base, out, *sets = sys.argv[1:]
d = yaml.safe_load(open(base))
for s in sets:
    path, val = s.split("=", 1)
    node = d["design"]
    for k in path.split("."):
        node = node[k]
    v = yaml.safe_load(val)
    if "range" in node and node.get("type", "").startswith("select") and v is not None and v not in node["range"]:
        sys.exit(f"design: {path}={v} is not one of {node['range']}")
    node["v"] = v
yaml.safe_dump(d, open(out, "w"), sort_keys=False)
print("design:", out, " ".join(sets))
