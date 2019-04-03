function fib(n)
   local f0, f1 = 0, 1
   if n == 0 then
      return f0
   elseif n == 1 then
      return f1
   else
      local i = 1
      local new_val = 0
      while i < n do
         new_val = f0 + f1
         f0 = f1
         f1 = new_val
         i = i + 1
      end
      return f1
   end
end

fib(60)
