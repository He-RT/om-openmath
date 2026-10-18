import type {NotebookFile} from '../kernel/generated/NotebookFile';
import type {NotebookState} from '../kernel/generated/NotebookState';
import type {KernelConfig} from '../kernel/generated/KernelConfig';
import type {CellOutput} from '../kernel/generated/CellOutput';
import type {CellStatus} from '../kernel/generated/CellStatus';
import type {Diagnostic} from '../kernel/generated/Diagnostic';
import type {Dialect} from '../kernel/generated/Dialect';
import type {FunctionDescriptor} from '../kernel/generated/FunctionDescriptor';
export interface FunctionDocument {descriptor:FunctionDescriptor;category:string;signature:string;wolfram_signature:string;summary_zh:string;summary_en:string;examples:{source:string;dialect:Dialect;verification:'not_verified'|'kernel_test'|'release_fixture'}[]}
export type MathCommand=
  |{type:'catalog'}
  |{type:'source_preview';source:string;dialect:Dialect}
  |{type:'preview';file:NotebookFile}
  |{type:'plan';cell_ids:string[];scope:'selected'|'required'|'affected';allowed_cell_ids:string[]}
  |{type:'run';cell_id:string}
  |{type:'scratch';source:string;dialect:Dialect;use_notebook_definitions:boolean};
export interface MathWork {expected:NotebookFile;general:KernelConfig['general'];command:MathCommand}
export type MathResult=
  |{type:'source_preview';diagnostics:Diagnostic[]}
  |{type:'catalog';catalog:{metadata_version:number;functions:FunctionDocument[]};context:{known_functions:string[];owned:{symbol:string;cell_id:string}[]}}
  |{type:'preview';affected_cell_ids:string[];analysis:{cells:{cell_id:string;defines:string[];uses:string[];function_names:string[];diagnostics:Diagnostic[]}[];order:string[];cycles:string[];blocked:string[];conflicts:{symbol:string;cell_ids:string[]}[];edges:[string,string][]}}
  |{type:'plan';plan:{requested_cell_ids:string[];ordered_cell_ids:string[];edges:[string,string][];cycle_cell_ids:string[];blocked_cell_ids:string[];unavailable_prerequisite_cell_ids:string[]}}
  |{type:'ran';cell_id:string;status:CellStatus;output:CellOutput|null;state:NotebookState;outputId?:string}
  |{type:'scratch';status:CellStatus;output:CellOutput|null;notebook_modified:false};
