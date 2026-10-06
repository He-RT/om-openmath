import {act,fireEvent,render,screen,waitFor} from '@testing-library/react';
import {expect,test,vi} from 'vitest';
import {ArtifactButton} from './ArtifactButton';
import {MockKernel} from '../../test/mockKernel';
import {messages} from '../../i18n';
import type {Response} from '../../kernel/client';
import {saveArtifact} from '../../state/files';
vi.mock('../../state/files',()=>({saveArtifact:vi.fn(async()=>true)}));
test('artifact export waits for actual bytes and persistence, while stale replies never save',async()=>{
 let done!:(v:Response)=>void;const kernel=new MockKernel(()=>new Promise(resolve=>done=resolve));const t=messages('en');
 const props={kernel,t,title:'data',label:'Export JSON',version:'old',enabled:true,request:()=>({type:'export_value_token' as const,format:'json' as const,token:'actual captured value'})};
 const view=render(<ArtifactButton {...props}/>);fireEvent.click(screen.getByRole('button',{name:'Export JSON'}));
 expect(saveArtifact).not.toHaveBeenCalled();view.rerender(<ArtifactButton {...props} version="new"/>);
 await act(async()=>done({type:'artifact',artifact:{mime:'application/json;charset=utf-8',extension:'json',byte_len:2,base64:'e30='}}));expect(saveArtifact).not.toHaveBeenCalled();
 fireEvent.click(screen.getByRole('button',{name:'Export JSON'}));await act(async()=>done({type:'artifact',artifact:{mime:'application/json;charset=utf-8',extension:'json',byte_len:2,base64:'e30='}}));
 await waitFor(()=>expect(screen.getByText('Download started')).toBeTruthy());expect(saveArtifact).toHaveBeenCalledTimes(1);
});
