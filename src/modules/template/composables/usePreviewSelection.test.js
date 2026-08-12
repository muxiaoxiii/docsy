import { describe, expect, it } from 'vitest'
import { ref } from 'vue'
import { usePreviewSelection } from './usePreviewSelection.js'

describe('usePreviewSelection 已保存模板预览', () => {
  it('完整替换内部 ref 占位符，不泄漏代码或重复字段原文', () => {
    const runs = [
      { id: 'plain-1', paragraphIndex: 0, text: '请求人' },
      { id: 'requester-run', paragraphIndex: 0, text: '{{fld_requester.ref.1}}' },
      { id: 'plain-2', paragraphIndex: 0, text: '与被请求人' },
      { id: 'respondent-run', paragraphIndex: 0, text: '{{fld_respondent.ref.1}}' },
      { id: 'plain-3', paragraphIndex: 0, text: '之间发生纠纷，委托' },
      { id: 'lawyer-1-run', paragraphIndex: 0, text: '{{fld_party_list_79d05910.ref.1}}' },
      { id: 'plain-4', paragraphIndex: 0, text: '律师、' },
      { id: 'lawyer-2-run', paragraphIndex: 0, text: '{{fld_party_list_79d05910.ref.2}}' },
      { id: 'plain-5', paragraphIndex: 0, text: '律师。' },
    ]
    const rows = [
      {
        rowId: 'edit:requester',
        enabled: true,
        type: 'text',
        name: '请求人',
        label: '请求人',
        text: '请求人',
        _fromManifestRef: true,
        markRefs: [{ markId: 'old-requester-run', start: 0, end: 3, tag: 'fld_requester.ref.1' }],
      },
      {
        rowId: 'edit:respondent',
        enabled: true,
        type: 'text',
        name: '对方',
        label: '对方',
        text: '对方',
        _fromManifestRef: true,
        markRefs: [{ markId: 'old-respondent-run', start: 0, end: 2, tag: 'fld_respondent.ref.1' }],
      },
      {
        rowId: 'edit:lawyers',
        enabled: true,
        type: 'party_list',
        name: '受托人',
        label: '受托人',
        text: '受托人',
        _fromManifestRef: true,
        markRefs: [
          { markId: 'old-lawyer-1-run', start: 0, end: 3, tag: 'fld_party_list_79d05910.ref.1' },
          { markId: 'old-lawyer-2-run', start: 0, end: 3, tag: 'fld_party_list_79d05910.ref.2' },
        ],
      },
    ]
    const state = usePreviewSelection(ref(runs), ref(''), ref(rows), {})
    const preview = state.buildTemplatePreview(runs, '', rows, {
      受托人: ['张三', '李四'],
    })
    const rendered = preview.rendered.map((segment) => segment.text).join('')

    expect(rendered).not.toContain('{{')
    expect(rendered).not.toContain('.ref.')
    expect(rendered).toContain('请求人【待填：请求人】与被请求人【待填：对方】')
    expect(rendered).toContain('委托张三律师、李四律师。')
  })
})
