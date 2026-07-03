import { marked } from 'marked'

marked.setOptions({ breaks: true, gfm: true })

// ponytail: no sanitizer — single self-hosted user, own content. DOMPurify wrap here if ever shared.
export function renderMarkdown(src) {
  if (!src) return ''
  return marked.parse(src)
}
