<script>
  import { api } from '../lib/api.js'
  import Modal from '../lib/components/Modal.svelte'
  import { faviconUrl, linkHost, parseTags, allTags, filterLinks, groupByCategory } from '../lib/links.js'

  const FIELD = 'w-full rounded-sm border border-border bg-surface-2 px-2.5 py-2 font-mono text-[13px] text-ink placeholder:text-faint focus:border-accent focus:shadow-[0_0_0_3px_rgba(62,245,196,0.14)] focus:outline-none'

  let links   = $state([])
  let error   = $state('')
  let query   = $state('')
  let active  = $state([]) // active tag filter (AND)

  let showForm = $state(false)
  let editId   = $state(null)
  let fUrl     = $state('')
  let fTitle   = $state('')
  let fDesc    = $state('')
  let fCategory = $state('')
  let fTags    = $state('')
  let fError   = $state('')
  let fBusy    = $state(false)

  const tags = $derived(allTags(links))
  const groups = $derived(groupByCategory(filterLinks(links, query, active)))
  const shownCount = $derived(groups.reduce((n, g) => n + g.links.length, 0))

  async function load() {
    error = ''
    try { links = await api.get('/api/links') }
    catch (e) { error = e.message }
  }

  $effect(() => { load() })

  function toggleTag(t) {
    active = active.includes(t) ? active.filter((x) => x !== t) : [...active, t]
  }

  function openNew() {
    editId = null
    fUrl = ''; fTitle = ''; fDesc = ''; fCategory = ''; fTags = ''
    fError = ''
    showForm = true
  }

  function openEdit(l) {
    editId = l.id
    fUrl = l.url
    fTitle = l.title
    fDesc = l.description ?? ''
    fCategory = l.category ?? ''
    fTags = (l.tags || []).join(', ')
    fError = ''
    showForm = true
  }

  async function deleteLink(id, ev) {
    try { await api.del('/api/links/' + id); await load() }
    catch (e) { error = e.message }
  }

  async function toggleFavorite(l) {
    try {
      await api.put('/api/links/' + l.id, { ...l, favorite: !l.favorite })
      await load()
    } catch (e) { error = e.message }
  }

  async function save(e) {
    e.preventDefault()
    fError = ''
    const url = fUrl.trim()
    if (!url) { fError = 'url is required'; return }
    if (!(url.startsWith('http://') || url.startsWith('https://'))) {
      fError = 'url must start with http:// or https://'; return
    }
    fBusy = true
    try {
      const payload = {
        url,
        title: fTitle.trim() || null,
        description: fDesc.trim() || null,
        category: fCategory.trim() || null,
        tags: parseTags(fTags),
      }
      if (editId) await api.put('/api/links/' + editId, payload)
      else await api.post('/api/links', payload)
      showForm = false
      await load()
    } catch (err) { fError = err.message }
    finally { fBusy = false }
  }
</script>

<!-- Header -->
<div class="rise mb-6">
  <div class="flex flex-wrap items-center gap-4">
    <div>
      <h2 class="font-mono text-[15px] font-bold text-ink">
        <span class="text-accent glow-text">&gt;</span> links
      </h2>
      <p class="mt-0.5 font-mono text-[12px] text-faint">// bookmarks, by category and tag</p>
    </div>
    <button
      onclick={openNew}
      class="ml-auto h-8 rounded-sm bg-accent px-3 font-mono text-[13px] font-bold text-on-accent transition glow-soft hover:brightness-110"
    >+ new link</button>
  </div>

  {#if error}
    <p class="mt-4 rounded-sm border border-st-lost/30 bg-st-lost/10 px-3 py-2 font-mono text-[12px] text-st-lost">[ ERR ] {error}</p>
  {/if}
</div>

<!-- Search + tag chips -->
<div class="rise mb-5 flex flex-col gap-3" style="animation-delay:40ms">
  <input bind:value={query} placeholder="search title / url / description…" class={FIELD} />
  {#if tags.length}
    <div class="flex flex-wrap items-center gap-2">
      {#each tags as t (t)}
        <button
          onclick={() => toggleTag(t)}
          class="rounded-sm border px-2.5 py-1 font-mono text-[12px] transition {active.includes(t)
            ? 'border-accent text-accent shadow-[0_0_8px_rgba(62,245,196,0.18)]'
            : 'border-border text-faint hover:border-border-2 hover:text-muted'}"
        >#{t}</button>
      {/each}
      <span class="ml-auto font-mono text-[12px] text-faint tabular-nums">[{shownCount}]</span>
    </div>
  {/if}
</div>

<!-- Grouped list -->
<div class="rise flex flex-col gap-6" style="animation-delay:80ms">
  {#if shownCount === 0}
    <p class="font-mono text-[12px] text-faint">no links</p>
  {:else}
    {#each groups as g (g.category)}
      <div>
        <p class="mb-2 font-mono text-[12px] font-bold text-muted">
          {g.category || 'uncategorized'} <span class="text-faint tabular-nums">[{g.links.length}]</span>
        </p>
        <div class="flex flex-col gap-2">
          {#each g.links as l (l.id)}
            <div class="group flex items-start gap-3 rounded-sm border border-border bg-surface p-3 transition-all hover:border-accent-dim hover:shadow-[0_0_14px_rgba(62,245,196,0.12)]">
              <!-- Favicon: falls back to a block-cursor glyph on load error. -->
              <img
                src={faviconUrl(l.url)}
                alt=""
                width="16"
                height="16"
                class="mt-0.5 h-4 w-4 shrink-0 rounded-[2px]"
                onerror={(e) => { e.currentTarget.replaceWith(document.createTextNode('▸')) }}
              />
              <div class="min-w-0 flex-1">
                <a
                  href={l.url}
                  target="_blank"
                  rel="noopener noreferrer"
                  class="block truncate font-mono text-[13px] text-ink hover:text-accent"
                >{l.title}</a>
                <p class="truncate font-mono text-[11px] text-faint">{linkHost(l.url)}</p>
                {#if l.description}
                  <p class="mt-1 font-mono text-[12px] text-muted">{l.description}</p>
                {/if}
                {#if l.tags?.length}
                  <div class="mt-1.5 flex flex-wrap gap-1.5">
                    {#each l.tags as t (t)}
                      <span class="rounded-sm border border-border-2 px-1.5 py-0.5 font-mono text-[10px] text-faint">#{t}</span>
                    {/each}
                  </div>
                {/if}
              </div>
              <div class="flex shrink-0 items-center gap-1">
                <button
                  onclick={() => toggleFavorite(l)}
                  aria-label={l.favorite ? 'Unfavorite link' : 'Favorite link'}
                  class="grid h-10 w-10 place-items-center font-mono text-[15px] leading-none transition md:h-auto md:w-auto md:px-1 {l.favorite ? 'text-accent glow-text' : 'text-faint hover:text-accent'}"
                >{l.favorite ? '★' : '☆'}</button>
                <button
                  onclick={() => openEdit(l)}
                  aria-label="Edit link"
                  class="grid h-10 w-10 place-items-center font-mono text-[13px] text-faint transition hover:text-accent md:h-auto md:w-auto md:px-1"
                >edit</button>
                <button
                  onclick={(e) => deleteLink(l.id, e)}
                  aria-label="Delete link"
                  class="grid h-10 w-10 place-items-center font-mono text-[16px] leading-none text-faint transition hover:text-st-lost"
                >×</button>
              </div>
            </div>
          {/each}
        </div>
      </div>
    {/each}
  {/if}
</div>

<!-- New / edit modal -->
{#if showForm}
  <Modal title={editId ? 'Edit link' : 'New link'} onclose={() => (showForm = false)}>
    <form onsubmit={save} class="flex flex-col gap-3">
      {#if fError}
        <p class="rounded-sm border border-st-lost/30 bg-st-lost/10 px-3 py-2 font-mono text-[12px] text-st-lost">[ ERR ] {fError}</p>
      {/if}
      <div>
        <p class="mb-1.5 font-mono text-[11px] text-faint">url</p>
        <input bind:value={fUrl} placeholder="https://…" class={FIELD} />
      </div>
      <div>
        <p class="mb-1.5 font-mono text-[11px] text-faint">title <span class="text-faint">(defaults to host)</span></p>
        <input bind:value={fTitle} placeholder="optional" class={FIELD} />
      </div>
      <div>
        <p class="mb-1.5 font-mono text-[11px] text-faint">description</p>
        <input bind:value={fDesc} placeholder="optional" class={FIELD} />
      </div>
      <div>
        <p class="mb-1.5 font-mono text-[11px] text-faint">category</p>
        <input bind:value={fCategory} placeholder="optional (e.g. Rust)" class={FIELD} />
      </div>
      <div>
        <p class="mb-1.5 font-mono text-[11px] text-faint">tags <span class="text-faint">(comma-separated)</span></p>
        <input bind:value={fTags} placeholder="reference, free" class={FIELD} />
      </div>
      <button
        type="submit"
        disabled={fBusy}
        class="h-9 rounded-sm bg-accent font-mono text-[13px] font-bold text-on-accent transition glow-soft hover:brightness-110 disabled:opacity-50"
      >{editId ? 'save changes' : 'save link'}</button>
    </form>
  </Modal>
{/if}
