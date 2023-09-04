def fac_inner(fac, n, acc):
    return acc if n < 2 else fac(fac, n-1, n*acc)
def fac(n):
    return fac_inner(fac_inner, n, 1)
print(fac(1000))
