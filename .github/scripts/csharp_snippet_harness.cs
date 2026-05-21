using Microsoft.EntityFrameworkCore;
using ParadeDB.EntityFrameworkCore.Extensions;

var connectionString =
    $"Host={Environment.GetEnvironmentVariable("PARADEDB_HOST") ?? "localhost"};"
    + $"Port={Environment.GetEnvironmentVariable("PARADEDB_PORT") ?? "5432"};"
    + $"Database={Environment.GetEnvironmentVariable("PARADEDB_DATABASE") ?? "postgres"};"
    + $"Username={Environment.GetEnvironmentVariable("PARADEDB_USER") ?? "postgres"};"
    + $"Password={Environment.GetEnvironmentVariable("PARADEDB_PASSWORD") ?? "postgres"}";

var options = new DbContextOptionsBuilder<SnippetDbContext>()
    .UseNpgsql(connectionString, o => o.UseParadeDb())
    .Options;

await using var dbContext = new SnippetDbContext(options);

// __PARADEDB_SNIPPET__

public sealed class SnippetDbContext(DbContextOptions<SnippetDbContext> options)
    : DbContext(options)
{
    public DbSet<MockItem> MockItems => Set<MockItem>();

    protected override void OnModelCreating(ModelBuilder modelBuilder)
    {
        modelBuilder.Entity<MockItem>(entity =>
        {
            entity.ToTable("mock_items");
            entity.HasKey(item => item.Id);
            entity.Property(item => item.Id).HasColumnName("id");
            entity.Property(item => item.Description).HasColumnName("description");
            entity.Property(item => item.Rating).HasColumnName("rating");
            entity.Property(item => item.Category).HasColumnName("category");
        });
    }
}

public sealed class MockItem
{
    public int Id { get; set; }
    public string Description { get; set; } = "";
    public int Rating { get; set; }
    public string Category { get; set; } = "";
}
