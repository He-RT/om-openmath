"""从 functions.toml 的真实 runtime 描述生成共享 Rust 元数据；不自动注册回调。"""
from __future__ import annotations
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TYPES = {'expression', 'boolean', 'integer', 'real', 'enum', 'symbol'}
ATTRS = {'hold_all','hold_first','hold_rest','listable','protected','numeric_function','flat','orderless','one_identity'}


def render_runtime(catalog: dict) -> dict[Path, str]:
    owners = {n: f for f in catalog['functions'] for n in f['runtime_names']}
    entries, seen, aliases = [], set(), {}
    for r in catalog.get('runtime', []):
        name = r['name']
        if name not in owners or name in seen:
            raise ValueError(f'无效或重复实际回调：{name}')
        seen.add(name)
        f = owners[name]
        if not set(r['attributes']) <= ATTRS:
            raise ValueError(f'{name} 保持属性无效')
        if r['min'] < 0 or r.get('max', r['min']) < r['min']:
            raise ValueError(f'{name} 参数个数无效')
        if not r['options'] and not r.get('compatibility_syntax') and not any(p['variadic'] for p in r['parameters']):
            if sum(p['required'] for p in r['parameters']) != r['min'] or len(r['parameters']) != r.get('max'):
                raise ValueError(f'{name} 的固定位置参数描述与实际调用个数不一致')
        for alias in [r['modern_name']] + r['aliases']:
            if len(alias) < 2 or alias.lower() in aliases:
                raise ValueError(f'别名重复或保留单字母：{alias}')
            aliases[alias.lower()] = name
        params, options = [], []
        for source, target in [(r['parameters'],params), (r['options'],options)]:
            keys = set()
            for p in source:
                if p['name'] in keys or p['value_type'] not in TYPES:
                    raise ValueError(f'{name} 参数名或类型无效')
                keys.add(p['name'])
                if p.get('min', 0) > p.get('max', float('inf')):
                    raise ValueError(f'{name} 参数范围无效')
                if not isinstance(p['required'],bool) or not isinstance(p['held'],bool) or not isinstance(p['variadic'],bool):
                    raise ValueError(f'{name} 参数标志必须为布尔值')
                if not p['required'] and not (p.get('default_source') or p.get('default_context')):
                    raise ValueError(f'{name} 可选参数缺少真实默认说明')
                if p.get('default_source') in {'必填','省略','none'}:
                    raise ValueError(f'{name} 不能把文字占位作为默认值')
                target.append({k: p.get(k, default) for k,default in [
                    ('name',None),('runtime_name',None),('value_type',None),('required',None),
                    ('default_source',None),('default_context',None),('enum_values',[]),
                    ('min',None),('max',None),('variadic',False),('held',False)]})
        entries.append(dict(id=f['id'],canonical_name=f['name'],name=name,modern_name=r['modern_name'],
                            aliases=r['aliases'],arity={'min':r['min'],'max':r.get('max')},parameters=params,options=options,
                            compatibility_syntax=r.get('compatibility_syntax', []),attributes=r['attributes'],pipe_arg=f['pipe_arg'],effect_class=f['effect_class'],
                            support=f['current_support'],return_type=f['return_type'],precision_modes=f['precision_modes'],
                            precision_notes=f['precision_notes'],platforms=f['current_platforms']))
    if seen != set(owners):
        raise ValueError(f'缺少实际调用描述：{sorted(set(owners)-seen)}')
    payload=json.dumps(sorted(entries,key=lambda e:e['name']),ensure_ascii=False,indent=2)+'\n'
    return {ROOT/'crates/om-core/src/catalog/runtime.json': payload}
