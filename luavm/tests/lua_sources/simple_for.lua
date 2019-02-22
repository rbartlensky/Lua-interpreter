local a = 0
for i=1,20 do
   a = a + 1
end
assert(a == 20)
local a = 0
for i=20,1,1-2 do
   a = a + 1
end
assert(a == 20)
