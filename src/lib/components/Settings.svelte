<script lang="ts">
  import { settings } from "../stores/settings";

  let { onClose }: { onClose: () => void } = $props();

  let key = $state($settings.openaiKey);
  let model = $state($settings.openaiModel);

  function save() {
    settings.set({ openaiKey: key, openaiModel: model });
    onClose();
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="overlay" onclick={onClose}>
  <div class="modal" onclick={(e) => e.stopPropagation()}>
    <h2>Settings</h2>

    <label>
      OpenAI API Key
      <input
        type="password"
        placeholder="sk-..."
        bind:value={key}
      />
    </label>

    <label>
      Model
      <select bind:value={model}>
        <option value="gpt-4o-mini">gpt-4o-mini (Fast & Cheap)</option>
        <option value="gpt-4o">gpt-4o</option>
        <option value="gpt-4-turbo">gpt-4-turbo</option>
      </select>
    </label>

    <div class="actions">
      <button class="cancel" onclick={onClose}>Cancel</button>
      <button class="save" onclick={save}>Save</button>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }
  .modal {
    background: #1e1e2e;
    border: 1px solid #313244;
    border-radius: 12px;
    padding: 28px;
    width: 420px;
    display: flex;
    flex-direction: column;
    gap: 20px;
  }
  h2 {
    margin: 0;
    font-size: 18px;
    color: #cdd6f4;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 8px;
    font-size: 13px;
    color: #a6adc8;
  }
  input, select {
    background: #313244;
    border: 1px solid #45475a;
    color: #cdd6f4;
    border-radius: 8px;
    padding: 10px 12px;
    font-size: 14px;
    outline: none;
    font-family: inherit;
  }
  input:focus, select:focus {
    border-color: #89b4fa;
  }
  select option {
    background: #1e1e2e;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
  }
  .cancel {
    background: #313244;
    color: #cdd6f4;
    border: none;
    padding: 10px 20px;
    border-radius: 8px;
    cursor: pointer;
    font-size: 14px;
  }
  .cancel:hover { background: #45475a; }
  .save {
    background: #89b4fa;
    color: #1e1e2e;
    border: none;
    padding: 10px 20px;
    border-radius: 8px;
    cursor: pointer;
    font-size: 14px;
    font-weight: 600;
  }
  .save:hover { background: #74c7ec; }
</style>
