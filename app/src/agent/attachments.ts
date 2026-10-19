export interface AttachmentCard {id:string;name:string;size:number;type:string;sentImage:boolean}
export interface UserPresentation {text:string;references:{id:string;label:string}[];attachments:AttachmentCard[]}
export interface StoredAttachment {id:string;documentId:string;name:string;size:number;type:string;original:Blob;image?:{blob:Blob;mediaType:string;width:number;height:number}}
export function supportedImage(type:string){return ['image/png','image/jpeg','image/webp','image/gif'].includes(type);}
/** Preserve the original. Only a bounded raster derivative can be submitted to a model. */
export async function prepareAttachment(documentId:string,id:string,file:File,sendImage:boolean):Promise<StoredAttachment>{
  const attachment:StoredAttachment={id,documentId,name:file.name,size:file.size,type:file.type,original:file};
  if(!sendImage||!supportedImage(file.type))return attachment;
  if(file.size>40*1024*1024)throw new Error('Image exceeds the 40 MiB input limit; the draft was retained');
  const bitmap=await createImageBitmap(file);
  try{
    const ratio=Math.min(1,2048/Math.max(bitmap.width,bitmap.height)),canvas=document.createElement('canvas');canvas.width=Math.max(1,Math.round(bitmap.width*ratio));canvas.height=Math.max(1,Math.round(bitmap.height*ratio));
    const context=canvas.getContext('2d');if(!context)throw new Error('Image conversion unavailable');context.drawImage(bitmap,0,0,canvas.width,canvas.height);
    const blob=await new Promise<Blob>((resolve,reject)=>canvas.toBlob(value=>value?resolve(value):reject(new Error('Image conversion failed')),'image/png'));
    if(blob.size>20*1024*1024)throw new Error('Converted image exceeds the 20 MiB limit; the draft was retained');
    attachment.image={blob,mediaType:'image/png',width:canvas.width,height:canvas.height};return attachment;
  }finally{bitmap.close();}
}
export async function blobBase64(blob:Blob){const bytes=new Uint8Array(await blob.arrayBuffer());let binary='';for(let offset=0;offset<bytes.length;offset+=8192)binary+=String.fromCharCode(...bytes.subarray(offset,offset+8192));return btoa(binary);}
