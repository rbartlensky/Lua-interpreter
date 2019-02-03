function f()
   local x = 2
   assert(x == 2)
   function g()
      local x = 3
      assert(x == 3)
   end
   assert(x == 2)
end

f()
g()
