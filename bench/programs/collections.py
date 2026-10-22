xs = []
i = 0
while i < 300000:
    xs.append(i)
    i += 1
total = 0
for v in xs:
    total += v
print(total)
d = {}
k = 0
while k < 100000:
    d[k] = k * 2
    k += 1
print(len(d))
