"""从固定上游字体生成可复现的OFL导出标签字体；运行依赖fontTools 4.62.1，构建使用仓库内已生成资源。"""
from pathlib import Path
import hashlib,json,sys
from fontTools import subset
from fontTools.ttLib import TTFont
import fontTools
root=Path(__file__).resolve().parents[1]
assert fontTools.__version__=='4.62.1', '字体生成锁定fontTools 4.62.1'
source=root/'target/font-source/NotoSansSC-Regular.otf'
font=TTFont(source,recalcTimestamp=False)
options=subset.Options();options.recalc_timestamp=False;options.retain_gids=False;options.notdef_glyph=True;options.notdef_outline=True
options.name_IDs=['*'];options.name_legacy=True;options.name_languages=['*']
engine=subset.Subsetter(options=options)
chars=set(range(0x20,0x100))|set(range(0x370,0x400))|set(range(0x2000,0x2070))|set(range(0x2100,0x2300))|set(range(0x3000,0x3040))|set(range(0x4e00,0xa000))|set(range(0xff00,0xfff0))
engine.populate(unicodes=chars);engine.subset(font)
for record in font['name'].names:
 if record.nameID in [1,4,6,16,17]:
  name='OpenMath Plot Labels Regular' if record.nameID in [4,17] else 'OpenMathPlotLabels-Regular' if record.nameID==6 else 'OpenMath Plot Labels'
  record.string=name.encode(record.getEncoding(),errors='replace')
if 'CFF ' in font:
 font['CFF '].cff.fontNames=['OpenMathPlotLabels-Regular']
 top=font['CFF '].cff.topDictIndex[0];top.FullName='OpenMath Plot Labels Regular';top.FamilyName='OpenMath Plot Labels'
font['head'].modified=font['head'].created
out=root/'crates/om-kernel/assets/OpenMathPlotLabels-Regular.otf';font.save(out,reorderTables=True)
manifest={'upstream':'https://github.com/notofonts/noto-cjk','commit':'f8d157532fbfaeda587e826d4cd5b21a49186f7c','source_path':'Sans/SubsetOTF/SC/NotoSansSC-Regular.otf','source_sha256':hashlib.sha256(source.read_bytes()).hexdigest(),'fonttools':'4.62.1','asset_sha256':hashlib.sha256(out.read_bytes()).hexdigest(),'bytes':out.stat().st_size,'glyph_count':len(font.getGlyphOrder()),'license':'OFL-1.1','unicode_ranges':['0020-00FF','0370-03FF','2000-206F','2100-22FF','3000-303F','4E00-9FFF','FF00-FFEF']}
(root/'licenses/plot-font/manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(json.dumps(manifest,ensure_ascii=False))
