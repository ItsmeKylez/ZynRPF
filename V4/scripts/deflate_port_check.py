# Line-by-line Python port of the planned Rust inflate (puff-style), to validate the algorithm against real zlib.
import zlib, random
class E(Exception): pass
LBASE=[3,4,5,6,7,8,9,10,11,13,15,17,19,23,27,31,35,43,51,59,67,83,99,115,131,163,195,227,258]
LEXT=[0,0,0,0,0,0,0,0,1,1,1,1,2,2,2,2,3,3,3,3,4,4,4,4,5,5,5,5,0]
DBASE=[1,2,3,4,5,7,9,13,17,25,33,49,65,97,129,193,257,385,513,769,1025,1537,2049,3073,4097,6145,8193,12289,16385,24577]
DEXT=[0,0,0,0,1,1,2,2,3,3,4,4,5,5,6,6,7,7,8,8,9,9,10,10,11,11,12,12,13,13]
ORDER=[16,17,18,0,8,7,9,6,10,5,11,4,12,3,13,2,14,1,15]
class Bits:
    def __init__(s,d): s.d=d; s.pos=0; s.buf=0; s.cnt=0
    def need(s,n):
        while s.cnt<n:
            if s.pos>=len(s.d): raise E('trunc')
            s.buf|=s.d[s.pos]<<s.cnt; s.pos+=1; s.cnt+=8
        v=s.buf&((1<<n)-1); s.buf>>=n; s.cnt-=n; return v
def build(lengths):
    count=[0]*16
    for l in lengths: count[l]+=1
    left=1
    for ln in range(1,16):
        left<<=1; left-=count[ln]
        if left<0: raise E('oversub')
    offs=[0]*16
    for ln in range(1,15): offs[ln+1]=offs[ln]+count[ln]
    sym=[0]*len(lengths)
    for i,l in enumerate(lengths):
        if l: sym[offs[l]]=i; offs[l]+=1
    return count,sym
def decode(h,b):
    count,sym=h; code=first=index=0
    for ln in range(1,16):
        code|=b.need(1); c=count[ln]
        if code-c<first: return sym[index+(code-first)]
        index+=c; first+=c; first<<=1; code<<=1
    raise E('badcode')
def codes(b,out,lit,dist,mx):
    while True:
        s=decode(lit,b)
        if s<256:
            if len(out)>=mx: raise E('limit')
            out.append(s)
        elif s==256: return
        else:
            s-=257
            if s>=29: raise E('badsym')
            ln=LBASE[s]+b.need(LEXT[s])
            ds=decode(dist,b)
            if ds>=30: raise E('baddist')
            d=DBASE[ds]+b.need(DEXT[ds])
            if d>len(out): raise E('baddist')
            if len(out)+ln>mx: raise E('limit')
            st=len(out)-d
            for i in range(ln): out.append(out[st+i])
def inflate(data,mx):
    b=Bits(data); out=[]
    while True:
        last=b.need(1); ty=b.need(2)
        if ty==0:
            b.buf=0; b.cnt=0
            if b.pos+4>len(data): raise E('trunc')
            l=data[b.pos]|data[b.pos+1]<<8; nl=data[b.pos+2]|data[b.pos+3]<<8
            if l!=(~nl&0xFFFF): raise E('stored')
            b.pos+=4
            if b.pos+l>len(data): raise E('trunc')
            if len(out)+l>mx: raise E('limit')
            out+=data[b.pos:b.pos+l]; b.pos+=l
        elif ty==1:
            ll=[8]*144+[9]*112+[7]*24+[8]*8
            codes(b,out,build(ll),build([5]*30),mx)
        elif ty==2:
            nlen=b.need(5)+257; ndist=b.need(5)+1; ncode=b.need(4)+4
            if nlen>286 or ndist>30: raise E('counts')
            cl=[0]*19
            for i in range(ncode): cl[ORDER[i]]=b.need(3)
            lc=build(cl); lens=[0]*320; idx=0
            while idx<nlen+ndist:
                s=decode(lc,b)
                if s<16: lens[idx]=s; idx+=1
                else:
                    if s==16:
                        if idx==0: raise E('rep')
                        val=lens[idx-1]; rep=3+b.need(2)
                    elif s==17: val=0; rep=3+b.need(3)
                    else: val=0; rep=11+b.need(7)
                    if idx+rep>nlen+ndist: raise E('rep')
                    for _ in range(rep): lens[idx]=val; idx+=1
            if lens[256]==0: raise E('noeob')
            codes(b,out,build(lens[:nlen]),build(lens[nlen:nlen+ndist]),mx)
        else: raise E('blocktype')
        if last: break
    return bytes(out)
def stored(data):
    if not data: return bytes([1,0,0,255,255])
    o=bytearray(); ch=[data[i:i+65535] for i in range(0,len(data),65535)]
    for i,c in enumerate(ch):
        o.append(1 if i==len(ch)-1 else 0); l=len(c)
        o+=l.to_bytes(2,'little')+((~l)&0xFFFF).to_bytes(2,'little')+c
    return bytes(o)
def raw(data,level=6,strategy=zlib.Z_DEFAULT_STRATEGY):
    c=zlib.compressobj(level,zlib.DEFLATED,-15,8,strategy); return c.compress(data)+c.flush()
if __name__=='__main__':
    random.seed(1); n=0
    for t in range(600):
        k=random.choice([0,1,5,100,3000,70000])
        kind=random.choice(['rand','text','zeros','mix'])
        if kind=='rand': d=bytes(random.getrandbits(8) for _ in range(k))
        elif kind=='zeros': d=bytes(k)
        elif kind=='text': d=(b'<Item><name>wheel_%d</name>\n'%random.randint(0,99))*max(1,k//30)
        else: d=bytes(random.choice(b'abcab \n<>') for _ in range(k))
        for lvl,st in ((0,0),(1,0),(6,0),(9,0),(9,zlib.Z_FIXED),(9,zlib.Z_HUFFMAN_ONLY),(9,zlib.Z_RLE)):
            assert inflate(raw(d,lvl,st),10**9)==d,(kind,k,lvl,st); n+=1
        assert inflate(stored(d),10**9)==d; n+=1
        assert zlib.decompressobj(-15).decompress(stored(d))==d
    print('roundtrips ok',n)
    # hostile: never anything but E on garbage / truncations
    bad=0
    for t in range(20000):
        g=bytes(random.getrandbits(8) for _ in range(random.randint(0,40)))
        try: inflate(g,1<<16)
        except E: bad+=1
    z=raw(bytes(random.getrandbits(8) for _ in range(500))+b'x'*3000,9)
    for cut in range(len(z)):
        try: inflate(z[:cut],10**9)
        except E: pass
    try: inflate(raw(bytes(1<<20),9),1000); print('NO LIMIT')
    except E as e: print('bomb ->',e)
    print('garbage errors',bad)
