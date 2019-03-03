local a = 2
function f()
   local b = 3
   function g(c)
      a = a + b
   end
end
f()
g()
a = 5
g()
assert(a == 8)
