import {expect,test} from 'vitest';
import {defaultCamera,normalizePoint,matrices,project,zoom} from './camera';
test('camera display preserves mathematical world coordinates and projection center, with bounded zoom',()=>{
 const bounds:[[number,number,number],[number,number,number]]=[[100,200,-10],[102,202,-8]];
 expect(normalizePoint([101,201,-9],bounds)).toEqual([0,0,0]);
 const center=project([0,0,0],defaultCamera,1.6)!;expect(center[0]).toBeCloseTo(.5,12);expect(center[1]).toBeCloseTo(.5,12);
 expect(zoom(defaultCamera,.5).distance).toBe(3.25);expect(zoom(defaultCamera,1e100).distance).toBe(30);expect(zoom(defaultCamera,0)).toBe(defaultCamera);
 const n=matrices(defaultCamera,1.6).normal;for(let c=0;c<3;c++)expect(n[c*3]!**2+n[c*3+1]!**2+n[c*3+2]!**2).toBeCloseTo(1,6);
 expect(normalizePoint([1e200,2e200,3e200],[[1e200,2e200,3e200],[1.01e200,2.01e200,3.01e200]]).every(Number.isFinite)).toBe(true);
});
