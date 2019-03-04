local function f(a)
   return a, 1
end
local g = 5
local a = { [f(10)] = g; "x", "y"; x = 1, f(x)}
assert(a[10] == 5, "bla")
assert(a[1] == "x", "bla2")
assert(a[2] == "y", "bla3")
assert(a["x"] == 1)
assert(a[3] == nil)
-- XXX: add unpacking to table constructors
-- assert(a[4] == 1)
