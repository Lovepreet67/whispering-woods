let token = null;

// Handle login
document.getElementById("loginForm").addEventListener("submit", async (e) => {
  e.preventDefault();
  const username = document.getElementById("username").value;
  const password = document.getElementById("password").value;

  try {
    const res = await fetch("http://127.0.0.1:8080/auth/login", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ username, password })
    });
    const data = await res.json();

    if (data.Token) {
      token = data.Token;
      document.getElementById("loginContainer").style.display = "none";
      document.getElementById("dashboard").style.display = "block";
      fetchSnapshot();
      setInterval(fetchSnapshot, 5000);
    } else if (data.Error) {
      document.getElementById("loginError").innerText = data.Error;
    }
  } catch (err) {
    console.error("Login error:", err);
    document.getElementById("loginError").innerText = "Login failed";
  }
});

// Fetch snapshot with token
async function fetchSnapshot() {
  if (!token) return;

  try {
    const res = await fetch("http://127.0.0.1:8080/monitoring/snapshot", {
      headers: { "Authorization": token }
    });
    if (!res.ok) {
      console.error("Unauthorized or error fetching snapshot");
      return;
    }
    const snapshot = await res.json();

    // Nodes
    const nodes = Object.entries(snapshot.datanode_to_detail_map);
    const totalNodes = nodes.length;
    const activeNodes = nodes.filter(([_, n]) => n.is_active).length;
    const inactiveNodes = totalNodes - activeNodes;
    const totalStorage = nodes.reduce((acc, [_, n]) => acc + n.storage_remaining, 0) / (1024*1024);

    document.getElementById('totalNodes').innerText = totalNodes;
    document.getElementById('activeNodes').innerText = activeNodes;
    document.getElementById('inactiveNodes').innerText = inactiveNodes;
    document.getElementById('totalStorage').innerText = totalStorage.toFixed(2);

    // Files Table
    const filesBody = document.querySelector("#filesTable tbody");
    filesBody.innerHTML = "";
    for (const [fileName, chunkIds] of Object.entries(snapshot.file_to_chunk_map)) {
      let fileSize = 0;
      const chunkDetails = chunkIds.map(id => {
        const chunk = snapshot.chunk_id_to_detail_map[id];
        const size = chunk.end_offset - chunk.start_offset;
        fileSize += size;
        return id;
      }).join(", ");

      const row = document.createElement('tr');
      row.innerHTML = `<td>${fileName}</td><td>${fileSize}</td><td>${chunkDetails}</td>`;
      filesBody.appendChild(row);
    }

    // Chunks Table
    const chunksBody = document.querySelector("#chunksTable tbody");
    chunksBody.innerHTML = "";
    for (const [chunkId, chunk] of Object.entries(snapshot.chunk_id_to_detail_map)) {
      const size = chunk.end_offset - chunk.start_offset;
      const row = document.createElement('tr');
      row.innerHTML = `
        <td>${chunkId}</td>
        <td>${chunk.start_offset}</td>
        <td>${chunk.end_offset}</td>
        <td>${size}</td>
        <td>${chunk.state}</td>
        <td>${chunk.locations.join(", ")}</td>
      `;
      chunksBody.appendChild(row);
    }

  } catch (err) {
    console.error("Error fetching snapshot:", err);
  }
}
