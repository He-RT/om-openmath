import {render,screen} from '@testing-library/react';
import {expect,test,vi} from 'vitest';
import {SceneView} from './SceneView';
import {messages} from '../../i18n';
import {MockKernel} from '../../test/mockKernel';
import type {OutputItem} from '../../kernel/generated/OutputItem';
test('missing WebGL2 preserves original source and real OBJ export capability without displaying a fake image',()=>{
 vi.spyOn(HTMLCanvasElement.prototype,'getContext').mockReturnValue(null);
 const item:Extract<OutputItem,{type:'scene3_d'}>={type:'scene3_d',request:{kind:'parametric',expressions:['cos(t)','sin(t)','t'],axes:[{name:'t',range:[0,6]}],mesh_points:8,color:null,parameters:{}},data:{meshes:[],lines:[{positions:[[0,0,0],[1,1,1]],colors:[[.1,.5,.2,1],[.1,.5,.2,1]],width:2}],points:[],bounds:[[0,0,0],[1,1,1]],skipped:0,sampled:true},unavailable:null};
 render(<SceneView item={item} kernel={new MockKernel()} t={messages('en')} language="en"/>);
 expect(screen.getByRole('alert').textContent).toContain('cannot display');expect(screen.getByRole('button',{name:'Export OBJ'}).hasAttribute('disabled')).toBe(false);
 expect(screen.getByText('cos(t), sin(t), t')).toBeTruthy();vi.restoreAllMocks();
});
