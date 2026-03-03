// APP REPO: UI components
// The App Agent works freely here — no approval needed.

export function ItemCard({ item }) {
  return (
    <div className="item-card">
      <h3>{item.name}</h3>
      <p>{item.description}</p>
    </div>
  );
}

export function ItemList({ items, loading }) {
  if (loading) return <div className="spinner">Loading...</div>;
  if (!items.length) return <p>No items yet.</p>;
  return (
    <div className="item-list">
      {items.map(item => <ItemCard key={item.id} item={item} />)}
    </div>
  );
}
