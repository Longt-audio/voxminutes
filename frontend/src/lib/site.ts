/** 项目官网 / 发卡站（独角数卡）：用户可在此购买充值卡给积分充值。
 *  全局共享常量：设置页「关于我们」与欢迎弹窗 P2 领取成功区域共用。 */
export const OFFICIAL_WEBSITE_URL = 'https://voxmin.top'

/** 官网链接的展示文本（去协议头，短一些） */
export const OFFICIAL_WEBSITE_LABEL = 'voxmin.top'

/** 法律条款页面（官网上由 `voxminutes/legal/` 的文稿生成），首次启动的同意提示与
 *  设置页「关于我们」都指向这里。
 *
 *  官网目前只有中英两套法律页面；ja/ko 界面先落到英文页 ——
 *  等官网补上 ja/ko 翻译后，把 `legalLocale` 改成按语言返回即可（调用点无需改动）。
 *
 *  路径约定（与官网目录结构一致）：
 *    - 中文是站点根（`/` 就是中文首页，`/zh/` 只是跳转桩）→ 中文条款在 `/privacy.html`、`/terms.html`
 *    - 英文在 `/en/` 下 → `/en/privacy.html`、`/en/terms.html`
 *    - `/zh/privacy.html`、`/zh/terms.html` 也保留了跳转桩，老链接不会 404
 *
 *  ⚠️ 改域名/路径时两处要一起改：官网静态页、这里。 */
export function legalLocale(language: string): 'zh' | 'en' {
  return language === 'zh' ? 'zh' : 'en'
}

export function privacyPolicyUrl(language: string): string {
  const prefix = legalLocale(language) === 'zh' ? '' : '/en'
  return `${OFFICIAL_WEBSITE_URL}${prefix}/privacy.html`
}

export function termsOfServiceUrl(language: string): string {
  const prefix = legalLocale(language) === 'zh' ? '' : '/en'
  return `${OFFICIAL_WEBSITE_URL}${prefix}/terms.html`
}

/** 记录「用户已同意条款」的设置键：值为同意时的 ISO 时间戳。
 *  留痕用途（上架审核/监管问询时能证明告知与同意的时点），不参与业务逻辑。 */
export const LEGAL_CONSENT_SETTING_KEY = 'legal.consent_accepted_at'

/** 客服 / 退款 / 换机迁移的联系邮箱。
 *  与《用户协议》《隐私政策》里写的是同一个地址（改这里要同步改 legal/ 下的四份文稿和官网页面）。 */
export const SUPPORT_EMAIL = 'voxmin@qq.com'
