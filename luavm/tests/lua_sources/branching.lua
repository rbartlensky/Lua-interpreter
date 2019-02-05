local a, b, c = 0, 2, 3
assert(a == 0)
assert(b == 2)
assert(c == 3)
if b == 1 then
   a = 5
elseif b == 2 then
   a = 6
   if c == 3 then
      a = 7
   end
else
   a = 8
end
assert(a == 7)
