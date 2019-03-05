local function f(a, b)
   return a + b
end
do
   local function f(a, b)
      return a * b
   end
   assert(f(1, 2) == 2)
end
assert(f(1, 2) == 3)
