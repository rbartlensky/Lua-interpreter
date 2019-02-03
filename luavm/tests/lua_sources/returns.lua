function add(a, b)
   return a + b
end

local x = add(2, 3)
assert(x == 5)
assert(add(5, 5) == 10)

function vargs(...)
   return 1, ...
end

local a, b, c = vargs(2, 3, 4, 5)
assert(a == 1)
assert(b == 2)
assert(c == 3)

local y, z, w = vargs(add(1, 2), add(2, 3))
assert(y == 1)
assert(z == 3)
assert(w == 5)
