function f()
   local x = 3
   assert(x == 3)
   x = 4 + 1
   assert(x == 4)
   y = 5 + x
   assert(y == 9)
end
