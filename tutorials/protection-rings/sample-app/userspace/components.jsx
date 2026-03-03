// RING 3: UI components (userspace)
// The userspace agent can freely modify this file.
// No approval needed.

export function ItemCard({ item }) {
  return (
    <div className="item-card">
      <h3>{item.name}</h3>
      <p>{item.description}</p>
      <span className="item-date">{new Date(item.created_at).toLocaleDateString()}</span>
    </div>
  );
}

export function ItemList({ items, loading }) {
  if (loading) return <div className="spinner">Loading...</div>;
  if (!items.length) return <p className="empty">No items yet. Create one!</p>;
  return (
    <div className="item-list">
      {items.map(item => <ItemCard key={item.id} item={item} />)}
    </div>
  );
}

export function CreateItemForm({ onSubmit }) {
  return (
    <form onSubmit={onSubmit} className="create-form">
      <input name="name" placeholder="Item name" required />
      <textarea name="description" placeholder="Description" />
      <button type="submit">Create Item</button>
    </form>
  );
}
