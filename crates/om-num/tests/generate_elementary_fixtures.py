"""Offline Decimal oracles; Chudnovsky pi is independent of Rust's Machin series."""
from decimal import Decimal as D, localcontext, ROUND_FLOOR
from math import factorial
from pathlib import Path

ROOT = Path(__file__).resolve().parent

def pi():
    total = D(0)
    for k in range(90):
        total += D(factorial(6*k) * (13591409+545140134*k)) / D(factorial(3*k)*factorial(k)**3*(-262537412640768000)**k)
    return D(426880)*D(10005).sqrt()/total

def atan(x, p):
    if x < 0: return -atan(-x, p)
    if x > 1: return p/2 - atan(1/x, p)
    if x > D('0.5'): return p/4 + atan((x-1)/(x+1), p)
    term, result = x, x
    for k in range(1, 1000):
        term *= -x*x
        result += term/(2*k+1)
    return result

def trig(x,p,cos=False):
    x -= (x/(2*p)).to_integral_value()*2*p
    term = D(1) if cos else x
    result = term
    for k in range(1, 250):
        degree=2*k if cos else 2*k+1
        term *= -x*x / ((degree-1)*degree)
        result += term
    return result

with localcontext() as ctx:
    ctx.prec=1300
    p=pi()
    (ROOT/'pi_1000.txt').write_text(str(p)[:1002]+'\n')
    ctx.prec=180
    lines=['# function\texact rational input\tfloor(value * 10^100); generated offline with Decimal']
    for raw in ['-10','-3','-1','-0.01','0','0.25','1','3','10','100']:
        x=D(raw)
        functions={'exp':x.exp(),'atan':atan(x,p),'sin':trig(x,p),'cos':trig(x,p,True)}
        if x>0: functions['ln']=x.ln()
        for name,value in functions.items():
            # Avoid exact values: their certified balls may have zero radius.
            if value == value.to_integral_value(): continue
            floor=int((value*D(10)**100).to_integral_value(rounding=ROUND_FLOOR))
            num, den = x.as_integer_ratio()
            lines.append(f'{name}\t{num}/{den}\t{floor}')
    (ROOT/'elementary.tsv').write_text('\n'.join(lines)+'\n')
    ctx.prec=800
    lines=[]
    for exponent in [128,1000]:
        x=D(2)**exponent
        for name,value in [('sin',trig(x,p)),('cos',trig(x,p,True))]:
            floor=int((value*D(10)**100).to_integral_value(rounding=ROUND_FLOOR))
            lines.append(f'{name}\t{int(x)}/1\t{floor}')
    with (ROOT/'elementary.tsv').open('a') as file:
        file.write('\n'.join(lines)+'\n')
    ctx.prec=180
    lines=['# function\tre\tim\tcomponent\tfloor(value*10^100)\tupper offset (zero for exact integer)']
    for raw_re,raw_im in [('1','2'),('-3','-4'),('-3','4'),('0','-1'),('-2','0'),('0','0')]:
        x,y=D(raw_re),D(raw_im)
        r=(x*x+y*y).sqrt()
        u=((r+x)/2).sqrt()
        v=((r-x)/2).sqrt() * (-1 if y<0 else 1)
        cosh=(y.exp()+(-y).exp())/2
        sinh=(y.exp()-(-y).exp())/2
        values={'sqrt':(u,v),'exp':(x.exp()*trig(y,p,True),x.exp()*trig(y,p)),
                'sin':(trig(x,p)*cosh,trig(x,p,True)*sinh),
                'cos':(trig(x,p,True)*cosh,-trig(x,p)*sinh)}
        if r:
            angle=atan(y/x,p) if x else p/2*(-1 if y<0 else 1)
            if x<0: angle += p*(-1 if y<0 else 1)
            values['ln']=(r.ln(),angle)
        for name,pair in values.items():
            for component,value in zip(['re','im'],pair):
                floor=int((value*D(10)**100).to_integral_value(rounding=ROUND_FLOOR))
                offset=0 if value==value.to_integral_value() else 1
                lines.append(f'{name}\t{raw_re}\t{raw_im}\t{component}\t{floor}\t{offset}')
    (ROOT/'complex_elementary.tsv').write_text('\n'.join(lines)+'\n')
