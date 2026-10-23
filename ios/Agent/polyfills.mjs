// Data-only Web primitives required by the pinned schema validator in JavaScriptCore.
if (!globalThis.TextEncoder) globalThis.TextEncoder=class {
  encode(source='') {
    const bytes=[];
    for(const char of String(source)) {
      let n=char.codePointAt(0);if(n>=0xD800&&n<=0xDFFF)n=0xFFFD;
      if(n<0x80)bytes.push(n);else if(n<0x800)bytes.push(0xC0|(n>>6),0x80|(n&63));
      else if(n<0x10000)bytes.push(0xE0|(n>>12),0x80|((n>>6)&63),0x80|(n&63));
      else bytes.push(0xF0|(n>>18),0x80|((n>>12)&63),0x80|((n>>6)&63),0x80|(n&63));
    }
    return new Uint8Array(bytes);
  }
};
if (!globalThis.URL) globalThis.URL=class {
  constructor(input,base){this.parts=JSON.parse(nativeURL(String(input),base?String(base):''));if(!this.parts.href)throw new Error('Invalid schema URL');}
  get href(){return this.parts.href;}get hash(){return this.parts.hash;}get origin(){return this.parts.origin;}
  get protocol(){return this.parts.protocol;}get pathname(){return this.parts.pathname;}
  toString(){return this.href;}toJSON(){return this.href;}
};
if (!globalThis.structuredClone) globalThis.structuredClone=value=>JSON.parse(JSON.stringify(value));
