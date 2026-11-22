import curses,random,time
S=[(0,1,1,2,2,3,3,4),(1,1,2,1,3,1,2,2),(1,1,2,1,3,1,3,2),(1,1,2,1,2,2,3,2),(1,2,2,2,2,1,3,1),(1,1,2,1,2,2,3,2),(1,1,2,1,2,2,3,2)]
curses.wrapper(lambda s:g(s))
def g(s):
 s.nodelay(1);s.keypad(1);b=[[0]*10 for _ in range(20)];p=random.choice(S);x,y=3,0;t=time.time()
 while 1:
  s.erase();s.addstr(0,0,"score %d"%sum(map(sum,b)))
  for r in range(20):
   for c in range(10):
    if b[r][c]:s.addstr(r+1,c*2,"[]")
  for i in range(0,8,2):
   if y+p[i]<20:s.addstr(y+p[i]+1,(x+p[i+1])*2,"[]")
  s.refresh();k=s.getch()
  if k==ord("q"):return
  if k==curses.KEY_LEFT:x=max(0,x-1)
  if k==curses.KEY_RIGHT:x=min(8,x+1)
  if k==curses.KEY_UP:p=S[random.randrange(7)];x,y=3,0
  if k==curses.KEY_DOWN or time.time()-t>.5:
   t=time.time();y+=1
   if any(y+p[i]>19 or b[y+p[i]][x+p[i+1]] for i in range(0,8,2)):
    y-=1
    for i in range(0,8,2):b[y+p[i]][x+p[i+1]]=1
    for r in range(20):
     if all(b[r]):b.pop(r);b.insert(0,[0]*10)
    p=S[random.randrange(7)];x,y=3,0
