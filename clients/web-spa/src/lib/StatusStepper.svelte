<script lang="ts">
  import { TAHAP_APPROVAL, tahapAktif, menungguSiapa, labelStatus, toneStatus, sisaSla } from '../lib/labels';

  interface Props {
    status: string;
    slaDeadline?: string | null;
    ringkas?: boolean;
  }

  let { status, slaDeadline = null, ringkas = false }: Props = $props();

  const aktif = $derived(tahapAktif(status));
  const menunggu = $derived(menungguSiapa(status));
  const sla = $derived(sisaSla(slaDeadline));
  const ditolak = $derived(status === 'RETURNED');
</script>

{#if ringkas}
  <div class="ringkas">
    <span class="badge {toneStatus(status)}">{labelStatus(status)}</span>
    {#if sla}
      <span class="badge {sla.tone}">{sla.teks}</span>
    {/if}
  </div>
{:else}
  <div class="stepper" class:rejected={ditolak}>
    {#each TAHAP_APPROVAL as tahap, i (tahap.kunci)}
      <div
        class="tahap"
        class:selesai={i < aktif}
        class:aktif={i === aktif && !ditolak}
        class:ditolak-tahap={ditolak && i === 0}
      >
        <div class="bulatan">
          {#if i < aktif}✓{:else}{i + 1}{/if}
        </div>
        <div class="nama">{tahap.label}</div>
      </div>
      {#if i < TAHAP_APPROVAL.length - 1}
        <div class="garis" class:selesai={i < aktif}></div>
      {/if}
    {/each}
  </div>

  <div class="keterangan">
    <span class="badge {toneStatus(status)}">{labelStatus(status)}</span>
    {#if menunggu}
      <span class="muted">Menunggu: <strong>{menunggu}</strong></span>
    {/if}
    {#if sla}
      <span class="badge {sla.tone}">SLA {sla.teks}</span>
    {/if}
  </div>
{/if}

<style>
  .ringkas { display: flex; gap: 0.35rem; flex-wrap: wrap; }

  .stepper {
    display: flex;
    align-items: center;
    gap: 0;
    margin-bottom: 0.5rem;
  }

  .tahap {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.25rem;
    min-width: 62px;
  }

  .bulatan {
    width: 26px;
    height: 26px;
    border-radius: 50%;
    border: 2px solid var(--border);
    background: var(--surface);
    color: var(--text-muted);
    display: grid;
    place-items: center;
    font-size: 0.75rem;
    font-weight: 700;
  }

  .nama {
    font-size: 0.72rem;
    color: var(--text-muted);
    text-align: center;
  }

  .garis {
    flex: 1;
    height: 2px;
    background: var(--border);
    margin-bottom: 1.15rem;
  }

  .tahap.selesai .bulatan { background: var(--success); border-color: var(--success); color: #fff; }
  .tahap.selesai .nama { color: var(--text); }
  .garis.selesai { background: var(--success); }

  .tahap.aktif .bulatan {
    border-color: var(--brand);
    color: var(--brand);
    box-shadow: 0 0 0 3px var(--brand-light);
  }
  .tahap.aktif .nama { color: var(--brand-dark); font-weight: 600; }

  .tahap.ditolak-tahap .bulatan { border-color: var(--danger); color: var(--danger); }

  .keterangan {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    flex-wrap: wrap;
    font-size: 0.82rem;
  }
</style>
