s = ""
i = 0
while i < 20000:
    s = s + "x"
    i += 1
print(len(s))
count = 0
for c in s:
    count += 1
print(count)
