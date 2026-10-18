import systemTemplate from '../../../agent/prompts/generated/system.zh.md?raw';
import executeMode from '../../../agent/prompts/mode-exec.zh.md?raw';
import discussMode from '../../../agent/prompts/mode-discuss.zh.md?raw';
export function agentSystemPrompt(version:string,mode:'execute'|'discuss'){
  if(!/^[A-Za-z0-9.+-]{1,80}$/.test(version))throw new Error('Invalid product version');
  const text=systemTemplate.replaceAll('{{version}}',version).replaceAll('{{mode_line}}',(mode==='execute'?executeMode:discussMode).trim());
  if(/\{\{[^}]+\}\}/.test(text))throw new Error('Unresolved system prompt variable');
  return text+'\n\n当前宿主为 Web。工具返回的 geometry 是实际几何数据；未注册渲染图像时 preview 为 data_only，不能声称已看到图像。附件能力只以本轮实际输入为准。';
}
