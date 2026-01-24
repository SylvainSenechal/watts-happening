// Load and visualize Strava data
async function loadData() {
  try {
    const response = await fetch("data/activities.json");
    const activities = await response.json();

    // Hide loading, show dashboard
    document.getElementById("loading").style.display = "none";
    document.getElementById("dashboard").style.display = "block";

    // Calculate and display stats
    displayStats(activities);

    // Create visualizations
    createDistanceChart(activities);
    createPowerChart(activities);
  } catch (error) {
    document.getElementById("loading").textContent =
      "⚠️ No data found. Run the Rust fetcher first!";
    console.error("Error loading data:", error);
  }
}

function displayStats(activities) {
  const stats = calculateStats(activities);
  const statsContainer = document.getElementById("stats");

  const statCards = [
    { title: "Total Activities", value: stats.totalActivities },
    { title: "Total Distance", value: `${stats.totalDistance.toFixed(0)} km` },
    { title: "Total Time", value: `${stats.totalHours.toFixed(0)} hrs` },
    {
      title: "Avg Power",
      value: stats.avgPower ? `${stats.avgPower.toFixed(0)} W` : "N/A",
    },
    {
      title: "Avg Heart Rate",
      value: stats.avgHR ? `${stats.avgHR.toFixed(0)} bpm` : "N/A",
    },
    { title: "Max Speed", value: `${stats.maxSpeed.toFixed(1)} km/h` },
  ];

  statsContainer.innerHTML = statCards
    .map(
      (stat) => `
        <div class="stat-card">
            <h3>${stat.title}</h3>
            <div class="value">${stat.value}</div>
        </div>
    `,
    )
    .join("");
}

function calculateStats(activities) {
  return {
    totalActivities: activities.length,
    totalDistance: activities.reduce((sum, a) => sum + a.distance / 1000, 0),
    totalHours: activities.reduce((sum, a) => sum + a.moving_time / 3600, 0),
    avgPower: calculateAverage(activities, "average_watts"),
    avgHR: calculateAverage(activities, "average_heartrate"),
    maxSpeed: Math.max(...activities.map((a) => a.max_speed * 3.6)),
  };
}

function calculateAverage(activities, field) {
  const validActivities = activities.filter((a) => a[field]);
  if (validActivities.length === 0) return null;
  return (
    validActivities.reduce((sum, a) => sum + a[field], 0) /
    validActivities.length
  );
}

function createDistanceChart(activities) {
  const margin = { top: 20, right: 30, bottom: 40, left: 60 };
  const width = 1000 - margin.left - margin.right;
  const height = 400 - margin.top - margin.bottom;

  // Prepare data
  const data = activities
    .map((a) => ({
      date: new Date(a.start_date),
      distance: a.distance / 1000, // Convert to km
    }))
    .sort((a, b) => a.date - b.date);

  // Create SVG
  const svg = d3
    .select("#distance-chart")
    .attr("width", width + margin.left + margin.right)
    .attr("height", height + margin.top + margin.bottom)
    .append("g")
    .attr("transform", `translate(${margin.left},${margin.top})`);

  // Scales
  const x = d3
    .scaleTime()
    .domain(d3.extent(data, (d) => d.date))
    .range([0, width]);

  const y = d3
    .scaleLinear()
    .domain([0, d3.max(data, (d) => d.distance)])
    .range([height, 0]);

  // Line generator
  const line = d3
    .line()
    .x((d) => x(d.date))
    .y((d) => y(d.distance));

  // Add axes
  svg
    .append("g")
    .attr("transform", `translate(0,${height})`)
    .call(d3.axisBottom(x));

  svg.append("g").call(d3.axisLeft(y));

  // Add line
  svg
    .append("path")
    .datum(data)
    .attr("fill", "none")
    .attr("stroke", "#667eea")
    .attr("stroke-width", 2)
    .attr("d", line);

  // Add dots
  svg
    .selectAll("circle")
    .data(data)
    .enter()
    .append("circle")
    .attr("cx", (d) => x(d.date))
    .attr("cy", (d) => y(d.distance))
    .attr("r", 4)
    .attr("fill", "#764ba2");
}

function createPowerChart(activities) {
  const activitiesWithPower = activities.filter((a) => a.average_watts);

  if (activitiesWithPower.length === 0) {
    d3.select("#power-chart")
      .append("text")
      .attr("x", 100)
      .attr("y", 50)
      .text("No power data available")
      .style("font-size", "16px")
      .style("fill", "#999");
    return;
  }

  const margin = { top: 20, right: 30, bottom: 40, left: 60 };
  const width = 1000 - margin.left - margin.right;
  const height = 400 - margin.top - margin.bottom;

  // Create histogram bins
  const powerData = activitiesWithPower.map((a) => a.average_watts);
  const histogram = d3
    .histogram()
    .domain([0, d3.max(powerData)])
    .thresholds(20);

  const bins = histogram(powerData);

  // Create SVG
  const svg = d3
    .select("#power-chart")
    .attr("width", width + margin.left + margin.right)
    .attr("height", height + margin.top + margin.bottom)
    .append("g")
    .attr("transform", `translate(${margin.left},${margin.top})`);

  // Scales
  const x = d3
    .scaleLinear()
    .domain([0, d3.max(bins, (d) => d.x1)])
    .range([0, width]);

  const y = d3
    .scaleLinear()
    .domain([0, d3.max(bins, (d) => d.length)])
    .range([height, 0]);

  // Add axes
  svg
    .append("g")
    .attr("transform", `translate(0,${height})`)
    .call(d3.axisBottom(x));

  svg.append("g").call(d3.axisLeft(y));

  // Add bars
  svg
    .selectAll("rect")
    .data(bins)
    .enter()
    .append("rect")
    .attr("x", (d) => x(d.x0))
    .attr("y", (d) => y(d.length))
    .attr("width", (d) => x(d.x1) - x(d.x0) - 1)
    .attr("height", (d) => height - y(d.length))
    .attr("fill", "#667eea");
}

// Load data when page loads
loadData();
