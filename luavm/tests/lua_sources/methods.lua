local function f_m(self, a)
   return a
end
local a = { f = f_m }
assert(a:f(2) == 2)
