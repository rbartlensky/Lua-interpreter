function f(a, b)
   x = a + b
end
x = 1 or f(1, 2) -- f(1, 2) is not called because 1 == true
assert(x == 1)
