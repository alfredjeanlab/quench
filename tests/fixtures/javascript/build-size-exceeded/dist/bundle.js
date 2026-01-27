// This is an oversized bundle that exceeds the 500 byte threshold
// Extra content to make it larger than the limit

const config = {
  apiUrl: "https://api.example.com",
  timeout: 5000,
  retries: 3,
  debug: false,
  version: "1.0.0"
};

function fetchData(endpoint) {
  return fetch(config.apiUrl + endpoint)
    .then(response => response.json())
    .catch(error => {
      console.error("Failed to fetch:", error);
      throw error;
    });
}

function processData(data) {
  return data.map(item => ({
    id: item.id,
    name: item.name,
    processed: true
  }));
}

export { config, fetchData, processData };
