export const metadata = {
  title: 'agora',
  description: 'federated communication platform',
};

export default function rootlayout({
  children,
}: {
  children: react.reactnode;
}) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
