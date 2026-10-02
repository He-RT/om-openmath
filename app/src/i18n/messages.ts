import type { Message } from "../kernel/generated/Message";
import type { Locale } from ".";
// Only exact known static messages are translated; unknown/dynamic details survive.
const chinese: Record<string, Record<string, string>> = {
  "Solve::ifun": {
    "Inverse functions are being used by Solve, so some solutions may not be found; use Reduce for complete solution information.":
      "求解使用了反函数，可能未找到所有解；可使用 Reduce 查看完整解的信息。",
  },
  "Solve::nsmet": {
    "This transcendental equation has unsupported independent kernels or inverse branches.":
      "该超越方程包含尚不支持的独立函数组合或反函数分支。",
  },
  "Solve::svars": {
    "Equations leave some requested variables free.":
      "方程未确定部分待求变量的值。",
    "More variables than equations; solving the first variables in name order":
      "变量数多于方程数；将按名称顺序求解前面的变量。",
  },
};
export function messageText(message: Message, language: Locale) {
  return language === "zh-CN"
    ? (chinese[`${message.symbol}::${message.tag}`]?.[message.text] ??
        message.text)
    : message.text;
}
